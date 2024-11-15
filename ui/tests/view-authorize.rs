mod helper;

#[test]
fn default() {
    helper::write(
        "/view-authorize-default.html",
        tekitoi_ui::view::authorize::View::default().with_style_path("style.css"),
    );
}

#[test]
fn with_profiles() {
    let mut view = tekitoi_ui::view::authorize::View::default().with_style_path("style.css");
    let mut profiles = tekitoi_ui::view::authorize::profiles::Section::default();
    profiles.add_user("Alice".into(), "/login/alice".into());
    profiles.add_user("Bob".into(), "/login/bob".into());
    view.set_profiles(profiles);
    helper::write("/view-authorize-with-profiles.html", view);
}

#[test]
fn with_credentials() {
    let mut view = tekitoi_ui::view::authorize::View::default().with_style_path("style.css");
    let creds = tekitoi_ui::view::authorize::credentials::Section::new("/login");
    view.set_credentials(creds);
    helper::write("/view-authorize-with-credentials.html", view);
}

#[test]
fn with_all() {
    let mut view = tekitoi_ui::view::authorize::View::default().with_style_path("style.css");
    let mut profiles = tekitoi_ui::view::authorize::profiles::Section::default();
    profiles.add_user("Alice".into(), "/login/alice".into());
    profiles.add_user("Bob".into(), "/login/bob".into());
    view.set_profiles(profiles);
    let creds = tekitoi_ui::view::authorize::credentials::Section::new("/login");
    view.set_credentials(creds);
    helper::write("/view-authorize-with-all.html", view);
}

#[test]
fn with_all_and_error() {
    let mut view = tekitoi_ui::view::authorize::View::default().with_style_path("style.css");
    let mut profiles = tekitoi_ui::view::authorize::profiles::Section::default();
    profiles.add_user("Alice".into(), "/login/alice".into());
    profiles.add_user("Bob".into(), "/login/bob".into());
    view.set_profiles(profiles);
    let creds = tekitoi_ui::view::authorize::credentials::Section::new("/login");
    view.set_credentials(creds);
    view.set_error("Something went wrong...".into());
    helper::write("/view-authorize-with-all-and-error.html", view);
}
