### **A Simple PSN API wrapper**

#### This is a working in progress. It have limit API functions for now.

<br>

#### Usage:
add `psn_api_rs = { git = "https://github.com/fakeshadow/psn_api_rs" features = ["client"]}`  to cargo.toml.<br>  
remove `features = ["client"]` if you don't want to use build in http client.(See more in documentation)

<br>

#### Features:
Use actix-web-client as http client connecting to PSN network.<br>
Get psn user profile, trophies, games info <br>
Receive/send PSN messages.<br>
Get PSN store info.

<br>

##### [Documentation](https://docs.rs/psn_api_rs/0.1.0/psn_api_rs/)


<br>

##### Known limitaion:
Performance could be really bad when running on some windows system. 