use std::io::stdin;

use futures::{lazy, Future};

// tokio is needed in example as the crate is running async request.
use tokio::runtime::current_thread::Runtime;
use tokio::prelude::*;
use psn_api_rs::{MakeClientCall, PSN, PSNUser};

fn main() {
    println!(
        "Pleas input your refresh_token if you alreayd have one. Press enter to skip to next\r\n"
    );
    let mut refresh_token = String::new();
    let mut uuid = String::new();
    let mut two_step = String::new();

    stdin().read_line(&mut refresh_token).unwrap();

    trim(&mut refresh_token);

    if refresh_token.len() == 0 {
        println!("Please input your uuid and press enter to continue.\r\n
You can check this link below to see how to get one paired with a two_step code which will be needed later\r\n
https://tusticles.com/psn-php/first_login.html\r\n");

        stdin().read_line(&mut uuid).unwrap();
        trim(&mut uuid);

        println!("Please input your two_step code to continue.\r\n");

        stdin().read_line(&mut two_step).unwrap();
        trim(&mut two_step);
    }

    println!("Please wait for the PSN network to response. The program will panic if there is an error occur\r\n");


    let mut runtime = Runtime::new().unwrap();

    let psn: PSN = runtime.block_on(lazy(|| {
        // construct and a new PSN struct, add credentials and call auth to generate tokens which are need to call other PSN APIs.
        PSN::new()
            .refresh_token(refresh_token)   // <- If refresh_token is provided then it's safe to ignore uuid and two_step arg and call .auth() directly.
            .uuid(uuid) // <- uuid and two_step are used only when refresh_token is not working or not provided.
            .two_step(two_step)
            .auth()
    })).unwrap_or_else(|e| panic!("{:?}", e));

    println!(
        "Authentication Success! These are your token info from PSN network: {:?} \r\n",
        psn
    );

    let user: PSNUser = runtime.block_on(
        psn.get_user_profile("Hakoom")  // <- use the psn struct to call for user_profile.
    ).unwrap_or_else(|e| panic!("{:?}", e));

    println!(
        "Test finished. Got user info : {:?}",
        user
    );

    // psn struct is dropped at this point so it's better to store your access_token and refresh_token locally.
}

fn trim(s: &mut String) {
    if s.ends_with("\n") {
        s.pop();
        if s.ends_with("\r") {
            s.pop();
        }
    }
}
