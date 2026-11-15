use super::*;

#[test]
fn login_goes_through_the_request_queue() {
    let mut c = Client::new().unwrap();
    c.login_token("Roy", "tok");
    assert_eq!(c.info().state, State::LoggingIn);
    let req = c.take_request().expect("a login request");
    assert!(req.url.contains("retroachievements.org"), "{}", req.url);
    let post = req.post.clone().unwrap_or_default();
    assert!(post.contains("r=login2") && post.contains("u=Roy"), "{post}");
    c.complete(req.id, 200, br#"{"Success":true,"User":"Roy","DisplayName":"Roy","Token":"tok","Score":10,"SoftcoreScore":0,"Messages":0}"#);
    let info = c.info();
    assert_eq!(info.state, State::LoggedIn, "{info:?}");
    assert_eq!(info.user, "Roy");
}

#[test]
fn a_gb_rom_is_hashed_whole() {
    let mut c = Client::new().unwrap();
    c.login_token("Roy", "tok");
    let login = c.take_request().unwrap();
    c.complete(login.id, 200, br#"{"Success":true,"User":"Roy","DisplayName":"Roy","Token":"tok","Score":10,"SoftcoreScore":0,"Messages":0}"#);
    let rom = vec![0u8; 32 * 1024];
    c.load_game(Console::GameBoy, &rom);
    assert_eq!(c.info().state, State::Loading);
    let req = c.take_request().expect("a game id request");
    let post = req.post.unwrap_or_default();
    assert!(post.contains("r=achievementsets"), "{post}");
    // MD5 of 32 KiB of zeros, the whole ROM for Game Boy
    assert!(post.contains("m=bb7df04e1b0a2570657527a7e108ae23"), "{post}");
}

#[test]
fn a_bad_login_fails_cleanly() {
    let mut c = Client::new().unwrap();
    c.login_token("Roy", "nope");
    let req = c.take_request().unwrap();
    c.complete(req.id, 200, br#"{"Success":false,"Error":"Invalid token"}"#);
    let info = c.info();
    assert_eq!(info.state, State::Failed);
    assert!(info.error.contains("Invalid") || !info.error.is_empty());
}
