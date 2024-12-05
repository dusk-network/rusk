// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs::{self, read, File};
use std::io::prelude::*;
use std::io::{self, ErrorKind};
use std::path::PathBuf;

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use tracing::info;
use version_check::Version;

use crate::{
    extension, file_name, file_stem, get_rusk_circuits_dir, get_rusk_keys_dir,
    Theme,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Circuit {
    id: [u8; 32],
    id_str: String,
    circuit: Vec<u8>,
    metadata: Metadata,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
struct Metadata {
    plonk_version: Option<String>,
    name: Option<String>,
}

impl Circuit {
    /// Create a new [`Circuit`]
    pub fn new(
        circuit: Vec<u8>,
        plonk_version: String,
        name: Option<String>,
    ) -> io::Result<Self> {
        let id = compute_id(&circuit, &plonk_version)?;
        Ok(Self {
            id,
            id_str: hex::encode(id),
            circuit,
            metadata: Metadata {
                plonk_version: Some(plonk_version),
                name,
            },
        })
    }

    /// Attempt to create a new [`Circuit`] from local storage
    pub fn from_stored(id: [u8; 32]) -> io::Result<Self> {
        let mut file = get_rusk_circuits_dir()?;
        let id_str = hex::encode(id);
        file.push(&id_str);
        file.set_extension("cd");

        let circuit = match &file.exists() {
            true => read(file),
            false => {
                Err(io::Error::new(ErrorKind::NotFound, "Circuit not found"))
            }
        }?;

        let circuit = Self {
            id,
            id_str,
            circuit,
            metadata: Metadata::from_stored(&id)?,
        };

        if let Some(result) = circuit.check_id() {
            if !result {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "The stored circuit id is incorrect",
                ));
            }
        }

        Ok(circuit)
    }

    /// Attempts to create a new [`Circuit`] from local storage by searching
    /// for the circuit name in the local toml files
    pub fn from_name(name: impl AsRef<str>) -> io::Result<Self> {
        let id = search_id(name.as_ref())?;
        Circuit::from_stored(id)
    }

    /// Checks whether [`Circuit::id`] is correct.
    ///
    /// Note: The check can only be performed when the plonk-version is stored
    /// as metadata in the [`Circuit`]
    pub fn check_id(&self) -> Option<bool> {
        match self.plonk_version() {
            None => None,
            Some(version) => {
                let computed_id = compute_id(self.circuit(), version)
                    .expect("plonk-version of a stored circuit to be valid");
                Some(computed_id == *self.id())
            }
        }
    }

    /// Stores the circuit description and circuit metadata (if there is
    /// metadata) or updates it if it exists but is different from the
    /// description in the struct
    pub fn store(&self) -> io::Result<()> {
        // store matadata
        self.metadata.update_or_store(&self.id)?;

        // store circuit
        let mut file = get_rusk_circuits_dir()?;
        file.push(self.id_str());
        let cd_file = file.with_extension("cd");
        File::create(&cd_file)?.write_all(&self.circuit)?;
        info!(
            "{}   {}",
            Theme::default().info("Cached"),
            file_name(&cd_file)
                .expect("At this point we know that the file is valid")
        );

        Ok(())
    }

    /// Returns the compressed circuit
    pub fn circuit(&self) -> &[u8] {
        &self.circuit
    }

    /// Returns the circuit id
    pub fn id(&self) -> &[u8; 32] {
        &self.id
    }

    /// Returns the circuit id in a hexadecimal string
    pub fn id_str(&self) -> &str {
        &self.id_str
    }

    /// Returns the circuit name if it exists, defaulting to the id string if
    /// not.
    pub fn name(&self) -> &str {
        self.metadata.name().unwrap_or(self.id_str())
    }

    /// Returns the plonk version of the metadata
    pub fn plonk_version(&self) -> Option<&str> {
        self.metadata.plonk_version.as_deref()
    }

    /// Returns the compressed circuit
    pub fn get_compressed(&self) -> &[u8] {
        &self.circuit
    }

    /// Fetches the prover key if stored in the keys directory
    pub fn get_prover(&self) -> io::Result<Vec<u8>> {
        let mut file = get_rusk_keys_dir()?;
        file.push(self.id_str());
        file.set_extension("pk");

        let pk = match &file.exists() {
            true => read(file),
            false => {
                Err(io::Error::new(ErrorKind::NotFound, "ProverKey not found"))
            }
        }?;

        Ok(pk)
    }

    /// Fetches the verifier data if stored in the keys directory
    pub fn get_verifier(&self) -> io::Result<Vec<u8>> {
        let mut file = get_rusk_keys_dir()?;
        file.push(self.id_str());
        file.set_extension("vd");

        let vd = match &file.exists() {
            true => read(file),
            false => Err(io::Error::new(
                ErrorKind::NotFound,
                "VerifierData not found",
            )),
        }?;

        Ok(vd)
    }

    /// Feches the prover key and verifier data if stored in the keys directory
    pub fn get_keys(&self) -> io::Result<(Vec<u8>, Vec<u8>)> {
        Ok((self.get_prover()?, self.get_verifier()?))
    }

    /// Stores the given prover key and verifier data
    pub fn add_keys(&self, pk: Vec<u8>, vd: Vec<u8>) -> io::Result<()> {
        let mut file = get_rusk_keys_dir()?;
        file.push(self.id_str());

        let pk_file = file.with_extension("pk");
        let vd_file = file.with_extension("vd");

        File::create(pk_file)?.write_all(&pk)?;
        File::create(vd_file)?.write_all(&vd)?;

        Ok(())
    }

    /// Cleans all stored files associated with the [`Circuit`]
    pub fn clean(&self) -> io::Result<()> {
        // collect all files with the circuit id as the file stem in circuits
        // directory
        let circuit_files: Vec<PathBuf> =
            fs::read_dir(get_rusk_circuits_dir()?)?
                .flatten()
                .map(|entry| entry.path())
                .filter(|file| file_stem(file) == Some(self.id_str()))
                .collect();

        for file in circuit_files {
            info!(
                "{}   /circuits/{}",
                Theme::default().warn("Removing"),
                file_name(&file).expect("file should be valid")
            );
            fs::remove_file(file)?;
        }

        // collect all files with the circuit id as the file stem in keys
        // directory
        let keys_files: Vec<PathBuf> = fs::read_dir(get_rusk_keys_dir()?)?
            .flatten()
            .map(|entry| entry.path())
            .filter(|file| file_stem(file) == Some(self.id_str()))
            .collect();

        for file in keys_files {
            info!(
                "{}   /keys/{}",
                Theme::default().warn("Removing"),
                file_name(&file).expect("file should be valid")
            );
            fs::remove_file(file)?;
        }
        Ok(())
    }
}

impl Metadata {
    /// Create new [`Metadata`]
    fn new(plonk_version: Option<String>, name: Option<String>) -> Self {
        Self {
            plonk_version,
            name,
        }
    }

    /// Attempt to create [`Metadata`] from local storage
    fn from_stored(id: &[u8; 32]) -> io::Result<Self> {
        let mut file = get_rusk_circuits_dir()?;
        file.push(hex::encode(id));
        file.set_extension("toml");

        Metadata::from_file(&file)
    }

    /// Attempt to create [`Metadata`] from a given file path
    fn from_file(file: &PathBuf) -> io::Result<Self> {
        let mut metadata = Metadata::new(None, None);
        if file.exists() {
            let content = read(file)?;
            let content =
                std::str::from_utf8(content.as_slice()).map_err(|e| {
                    io::Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Couldn't read metadata for {:?}: {}",
                            file.file_name().expect("file exists"),
                            e
                        ),
                    )
                })?;
            metadata = toml::from_str(content).map_err(|e| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Couldn't parse metadata for {:?}: {}",
                        file.file_name().expect("file exists"),
                        e
                    ),
                )
            })?;
        }
        Ok(metadata)
    }

    /// Return name
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Store the circuit metadata or updates it if it is different from the
    /// stored version
    fn update_or_store(&self, id: &[u8; 32]) -> io::Result<()> {
        let stored = Metadata::from_stored(id)?;

        if self != &stored {
            return self.add(id);
        }

        Ok(())
    }

    /// Stores the [`Metadata`] without perfoming any checks
    fn add(&self, id: &[u8; 32]) -> io::Result<()> {
        let mut file = get_rusk_circuits_dir()?;
        file.push(hex::encode(id));
        file.set_extension("toml");

        let toml = toml::to_string(self).map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("Couldn't create string from metadata: {e}"),
            )
        })?;
        File::create(&file)?.write_all(toml.as_bytes())?;

        Ok(())
    }
}

fn compute_id(circuit: &[u8], plonk_version: &str) -> io::Result<[u8; 32]> {
    // parse plonk version
    let (major, mut minor, _) = match Version::parse(plonk_version) {
        Some(v) => v.to_mmp(),
        None => {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!("coudn't parse plonk version: {plonk_version}"),
            ))
        }
    };

    // ignore minor when major > 0
    if major > 0 {
        minor = 0;
    }

    // hash circuit description and plonk version to compute id
    let mut hasher = Hasher::new();
    hasher.update(circuit);
    hasher.update(&major.to_be_bytes());
    hasher.update(&minor.to_be_bytes());
    Ok(hasher.finalize().into())
}

fn search_id(name: &str) -> io::Result<[u8; 32]> {
    // gather all toml files with the correct metadata format that specify the
    // name we are looking for
    let circuits_dir = get_rusk_circuits_dir()?;
    let toml_files: Vec<PathBuf> = fs::read_dir(circuits_dir)?
        .flatten()
        .map(|entry| entry.path())
        // filter on "toml" extension
        .filter(|file| extension(file) == Some("toml"))
        // filter on correct name and fileformat
        .filter(|file| {
            let metadata = Metadata::from_file(file);
            match metadata {
                Err(_) => false,
                Ok(data) => match data.name {
                    Some(stored_name) => stored_name == name,
                    None => false,
                },
            }
        })
        .collect();

    // we are only continuing when we found exactly one file
    if toml_files.len() == 1 {
        let id_str = file_stem(&toml_files[0]).expect("file exists");
        let id = hex::decode(id_str).map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("Couldn't parse id from {id_str}: {e}"),
            )
        })?;

        // we are only continuing when the id is exactly 32 bytes long
        if id.len() == 32 {
            let mut buf = [0u8; 32];
            buf.copy_from_slice(&id[0..32]);
            return Ok(buf);
        }
    }

    Err(io::Error::new(
        ErrorKind::NotFound,
        format!("Couldn't find circuit id for {name}"),
    ))
}
