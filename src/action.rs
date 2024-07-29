use crate::components::home::Focus;

pub enum Action {
    InstallPackage(String),
    SearchPackage(String),
    Focus(Focus),
}
