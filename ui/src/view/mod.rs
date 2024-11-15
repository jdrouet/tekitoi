pub mod authorize;
pub mod error;
pub mod redirect;

pub trait View {
    fn render(self) -> String;
}
