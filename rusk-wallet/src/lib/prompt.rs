// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use blake3::Hash;
use requestty::Question;

/// Request the user to authenticate with a password
pub(crate) fn request_auth() -> Hash {
    let q = Question::password("password")
        .message("Please enter your wallet's password:")
        .mask('*')
        .build();
    let a = requestty::prompt_one(q).unwrap();
    let pwd = a.as_string().unwrap_or("").to_string();
    let pwd = blake3::hash(pwd.as_bytes());
    pwd
}

pub(crate) fn create_password() -> Hash {
    let mut pwd = String::from("");

    let mut pwds_match = false;
    while !pwds_match {
        // enter password
        let q = Question::password("password")
            .message("Enter a strong password for your wallet:")
            .mask('*')
            .build();
        let a = requestty::prompt_one(q).unwrap();
        let pwd1 = a.as_string().unwrap_or("").to_string();

        // confirm password
        let q = Question::password("password")
            .message("Please confirm your password:")
            .mask('*')
            .build();
        let a = requestty::prompt_one(q).unwrap();
        let pwd2 = a.as_string().unwrap_or("").to_string();

        // check match
        pwds_match = pwd1 == pwd2;
        if pwds_match {
            pwd = pwd1.to_string()
        } else {
            println!("Passwords don't match, please try again.");
        }
    }

    let pwd = blake3::hash(pwd.as_bytes());
    pwd
}

pub(crate) fn confirm_recovery_phrase(phrase: String) {
    // inform the user about the mnemonic phrase
    println!("The following phrase is essential for you to regain access to your wallet\nin case you lose access to this computer.");
    println!("Please print it or write it down and store it somewhere safe:");
    println!();
    println!("> {}", phrase);
    println!();

    // let the user confirm they have backed up their phrase
    loop {
        let q = requestty::Question::confirm("proceed")
            .message("Have you backed up your recovery phrase?")
            .build();
        let a = requestty::prompt_one(q).unwrap();
        if a.as_bool().unwrap() {
            return;
        }
    }
}

pub(crate) fn request_recovery_phrase() -> String {
    // let the user input the recovery phrase
    let q = Question::input("phrase")
        .message("Please enter the recovery phrase:")
        .build();
    let a = requestty::prompt_one(q).unwrap();
    let phrase = a.as_string().unwrap().to_string();
    phrase
}
