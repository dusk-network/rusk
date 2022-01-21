// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::lib::crypto::MnemSeed;
use requestty::Question;

/// Request the user to authenticate with a password
pub(crate) fn request_auth() -> String {
    let q = Question::password("password")
        .message("Please enter your wallet's password:")
        .mask('*')
        .build();
    let a = requestty::prompt_one(q).unwrap();
    let pwd = a.as_string().unwrap_or("").to_string();
    pwd
}

pub(crate) struct CreateData (pub String, pub MnemSeed);

/// Create a new wallet
pub(crate) fn create() -> Option<CreateData> {
    
    // info message
    println!("You are now about to set up a new wallet.");
    println!("Remember a single wallet can hold multiple addresses.");

    // let the user confirm they trust this machine
    let q = requestty::Question::confirm("proceed")
        .message("Do you trust this computer?")
        .build();
    let a = requestty::prompt_one(q).unwrap();
    if !a.as_bool().unwrap() {
        return None;
    }

    // info message
    //println!("We recommend giving your wallet a name.");
    //println!("We'll save your wallet with this name under your default data directory.");

    // get default user as default wallet name (remove whitespace)
    let mut user: String = whoami::username();
    user.retain(|c|!c.is_whitespace());

    // name the wallet file
    let q = Question::input("wallet_name")
        .message("Give you wallet a name:")
        .default(user)
        .build();
    let a = requestty::prompt_one(q).unwrap();
    let mut wallet_name = a.as_string().unwrap().to_string();
    wallet_name.retain(|c|!c.is_whitespace());

    // info message
    //println!();
    //println!("We are now going to secure this wallet.");
    //println!("It's important you chose a strong password that you remember.");

    // password input
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

    // generate mnemonic and seed
    let ms = MnemSeed::new(pwd);

    // info message
    println!();
    println!("The following phrase is essential for you to recover your wallet\nin case you lose access to this computer.");
    println!("Please print it or write it down and store it somewhere safe:");
    println!();
    println!("> {}", ms.phrase);
    println!();

    // let the user confirm they have backed up their phrase
    let q = requestty::Question::confirm("proceed")
        .message("Have you backed up your recovery phrase?")
        .build();
    let a = requestty::prompt_one(q).unwrap();
    if !a.as_bool().unwrap() {
        return None;
    }

    // return collected data
    Some(CreateData(wallet_name, ms))

}
