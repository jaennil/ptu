// use crate::{component::Component, search::PackageSearch};
//
// use ratatui::{
//     layout::{Constraint, Layout},
//     Frame,
// };
//
// #[derive(Default)]
// pub struct Components {
//     package_search: PackageSearch,
// }
//
// impl Components {
//     pub fn as_array(&self) -> [&dyn Component; 1] {
//         [&self.package_search]
//     }
// }
//
// pub struct UI {
    // package_input: PackageInput,
    // state: UIState,
    // package_search: PackageSearch,
//     pub components: Components,
// }

// #[derive(Default)]
// struct UIState {
// package_input: String,
// packages: Vec<Package>,
// }

// impl UI {
    // pub fn new() -> Self {
        // Self {
            // components,
            // state: Default::default(),
        // }
    // }

    // pub fn render(&self, f: &mut Frame) {
    //     let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).split(f.size());
        // f.render_widget(self.package_search, layout[0]);
//         self.components.package_search.render(f, layout[0]);
//     }
// }
