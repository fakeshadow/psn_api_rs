use std::io::stdin;

use futures::lazy;

use psn_api_rs::{PSNRequest, PSNUser, TrophySet, TrophyTitles, PSN};

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

    // tokio is needed in example as the crate is running async request.
    let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();

    let mut psn: PSN = runtime
        .block_on(lazy(|| {
            // construct and a new PSN struct, add credentials and call auth to generate tokens which are need to call other PSN APIs.
            PSN::new()
                .add_refresh_token(refresh_token) // <- If refresh_token is provided then it's safe to ignore uuid and two_step arg and call .auth() directly.
                .add_uuid(uuid) // <- uuid and two_step are used only when refresh_token is not working or not provided.
                .add_two_step(two_step)
                .auth()
        }))
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!(
        "Authentication Success! These are your token info from PSN network: {:?} \r\n",
        psn
    );

    let user: PSNUser = runtime
        .block_on(
            psn.add_online_id("Hakoom".to_owned()).get_profile(), // <- use the psn struct to call for user_profile
        )
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("Got example user info : {:?}", user);

    let titles: TrophyTitles = runtime
        .block_on(psn.add_online_id("Hakoom".to_owned()).get_titles(0))
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("Got example trophy titles info : {:?}", titles);

    let set: TrophySet = runtime
        .block_on(
            psn.add_online_id("Hakoom".to_owned())
                .add_np_communication_id("NPWR10788_00".to_owned())
                .get_trophy_set(),
        )
        .unwrap_or_else(|e| panic!("{:?}", e));

    println!("Got example trophy set info : {:?}", set);

    println!("Although the console is a mess, the test is finished and all api enpoints are good");
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

