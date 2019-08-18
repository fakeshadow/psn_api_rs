use std::io::stdin;

use futures::lazy;
use tokio::runtime::current_thread::Runtime;

use psn_api_rs::{
    models::{
        MessageThread, MessageThreadsSummary, PSNUser, StoreSearchResult, TrophySet, TrophyTitles,
    },
    PSNRequest, PSN,
};

fn main() {
    let mut refresh_token = String::new();
    let mut uuid = String::new();
    let mut two_step = String::new();

    println!(
        "Pleas input your refresh_token if you already have one. Press enter to skip to next\r\n"
    );

    stdin().read_line(&mut refresh_token).unwrap();
    trim(&mut refresh_token);

    if refresh_token.is_empty() {
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

    let mut psn: PSN = runtime
        .block_on(lazy(||
            // construct a new PSN struct, add args and call auth to generate tokens which are need to call other PSN APIs.
            PSN::new()
                .set_region("us".to_owned()) // <- set to a psn region server suit your case. you can leave it as default which is hk
                .set_lang("en".to_owned()) // <- set to a language you want the response to be. default is en
                .set_self_online_id(String::from("Your Login account PSN online_id")) // <- this is used to generate new message thread.
                // safe to leave unset if you don't need to send any PSN message.
                .add_refresh_token(refresh_token) // <- If refresh_token is provided then it's safe to ignore uuid and two_step arg and call .auth() directly.
                .add_uuid(uuid) // <- uuid and two_step are used only when refresh_token is not working or not provided.
                .add_two_step(two_step)
                .auth()))
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!(
        "\r\nAuthentication Success! You PSN info are:\r\n{:#?}",
        psn
    );

    // get psn user profile by online id
    let user: PSNUser = runtime
        .block_on(
            psn.add_online_id("Hakoom".to_owned()).get_profile(), // <- use the psn struct to call for user_profile
        )
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("\r\nGot example user info : \r\n{:#?}", user);

    // get psn user trophy lists by online id
    let titles: TrophyTitles = runtime
        .block_on(psn.add_online_id("Hakoom".to_owned()).get_titles(0))
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("\r\nGot example trophy titles info : \r\n{:#?}", titles);

    //get one game trophy detailed list by online id and game np communcation id
    let set: TrophySet = runtime
        .block_on(
            psn.add_online_id("Hakoom".to_owned())
                .add_np_communication_id("NPWR10788_00".to_owned())
                .get_trophy_set(),
        )
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("\r\nGot example trophy set info : \r\n{:#?}", set);

    //get self message threads
    let threads: MessageThreadsSummary = runtime
        .block_on(psn.get_message_threads(0))
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("\r\nGot example threads info : \r\n{:#?}", threads);

    //get the last message thread detail. if the account have at least one message thread.
    match threads.threads.first() {
        Some(t) => {
            let thread: MessageThread = runtime
                .block_on(psn.get_message_thread(t.thread_id.as_str()))
                .unwrap_or_else(|e| panic!("{:?}", e));

            println!("\r\nGot example thread detail info : \r\n{:#?}", thread);
        }
        None => println!("\r\nIt seems this account doesn't have any threads so thread detail example is skipped")
    }

    // store apis don't need authentication.
    let search: StoreSearchResult = runtime
        .block_on(PSN::new().search_store_items("en", "us", "20", "ace combat"))
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("Got example PSN store response: {:#?}", search);

    println!("\r\n\r\nThe example is finished and all api endpoints are good");
    println!("\r\n\r\npsn struct is dropped at this point so it's better to store your access_token and refresh_token locally to make sure they can be reused");
    println!("Your psn info is : {:#?}", psn);
}

fn trim(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}
