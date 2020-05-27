use std::io::stdin;

use psn_api_rs::{
    models::{
        MessageThread, MessageThreadsSummary, PSNUser, StoreSearchResult, TrophySet, TrophyTitles,
    },
    psn::PSN,
    traits::PSNRequest,
    types::PSNInner,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // string collector
    let (refresh_token, npsso) = collect_input();

    // build a temporary reqwest http client for initial authentication
    let client = PSN::new_client().expect("Failed to build http client");

    // construct a new PSNInner object, add args and call auth to generate tokens which are need to call other PSN APIs.
    let mut psn_inner = PSNInner::new();
    psn_inner
        .set_region("us".to_owned()) // <- set to a psn region server suit your case. you can leave it as default which is hk
        .set_lang("en".to_owned()) // <- set to a language you want the response to be. default is en
        .set_self_online_id(String::from("Your Login account PSN online_id")) // <- this is used to generate new message thread. safe to leave unset if you don't need to send any PSN message.
        .add_refresh_token(refresh_token) // <- If refresh_token is provided then it's safe to ignore add_npsso and call auth directly.
        .add_npsso(npsso); // <- npsso is used only when refresh_token is not working or not provided.

    psn_inner = psn_inner
        .auth(client.clone())
        .await
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!(
        "\r\nAuthentication Success! You PSN info are:\r\n{:#?}",
        psn_inner
    );

    // multiple PSNInner can be pass to PSN object
    let psn = PSN::new(vec![psn_inner]).await;

    // get psn user profile by online id
    let user: PSNUser = psn
        .get_profile("Hakoom")
        .await
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("\r\nGot examples user info : \r\n{:#?}", user);

    // get psn user trophy lists by online id
    let titles: TrophyTitles = psn
        .get_titles("Hakoom", 0)
        .await
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("\r\nGot examples trophy titles info : \r\n{:#?}", titles);

    //get one game trophy detailed list by online id and game np communication id
    let set: TrophySet = psn
        .get_trophy_set("Hakoom", "NPWR10788_00")
        .await
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("\r\nGot examples trophy set info : \r\n{:#?}", set);

    // get self message threads
    let threads: MessageThreadsSummary = psn
        .get_message_threads(0)
        .await
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("\r\nGot examples threads info : \r\n{:#?}", threads);

    // get the last message thread detail. if the account have at least one message thread.

    match threads.threads.first() {
        Some(t) => {
            let thread: MessageThread = psn
                .get_message_thread(t.thread_id.as_str())
                .await
                .unwrap_or_else(|e| panic!("{:?}", e));

            println!("\r\nGot examples thread detail info : \r\n{:#?}", thread);
        }
        None => println!("\r\nIt seems this account doesn't have any threads so thread detail examples is skipped")
    }

    // retrieve our new refresh_token from PSN
    let inners = psn.get_inner();
    let psn_inner = inners.get().await.unwrap();
    let refresh_token = psn_inner.get_refresh_token().map(String::from);
    drop(psn_inner);

    // store apis don't need authentication.
    let psn_inner = PSNInner::new();

    let psn = PSN::new(vec![psn_inner]).await;
    let search: StoreSearchResult = psn
        .search_store_items("en", "us", "20", "battlefield")
        .await
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("Got examples PSN store response: {:#?}", search);

    println!("\r\n\r\nThe examples is finished and all api endpoints are good");
    println!("\r\n\r\npsn struct is dropped at this point so it's better to store your refresh_token locally to make sure they can be reused");
    println!("Your (possible) new refresh_token is : {:#?}. You can use this refresh_token next time you try this example", refresh_token);

    Ok(())
}

// helper function to collect input
fn collect_input() -> (String, String) {
    let mut refresh_token = String::new();
    let mut npsso = String::new();

    println!(
        "Pleas input your refresh_token if you already have one. Press enter to skip to next\r\n"
    );

    stdin().read_line(&mut refresh_token).unwrap();
    trim(&mut refresh_token);

    if refresh_token.is_empty() {
        println!(
            "Please input your npsso and press enter to continue.\r\n
You can check this link below to see how to get one\r\n
https://tusticles.com/psn-php/first_login.html\r\n"
        );

        stdin().read_line(&mut npsso).unwrap();
        trim(&mut npsso);
    }

    if refresh_token.is_empty() && npsso.is_empty() {
        panic!("must provide refresh_token or npsso to proceed");
    }

    println!("Please wait for the PSN network to response. The program will panic if there is an error occur\r\n");

    (refresh_token, npsso)
}

fn trim(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}
