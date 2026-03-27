use crate::models::execution::ExecutionResult;
use crate::models::menu::MenuItem;

pub trait MenuItemProvider {
    fn get_menu_items(&self) -> Vec<MenuItem>;
    fn refresh(&mut self);
}

pub trait ExecutionHandler {
    fn can_handle(&self, item: &MenuItem) -> bool;
    fn execute(&self, item: &MenuItem, context: Option<&str>) -> ExecutionResult;
}
