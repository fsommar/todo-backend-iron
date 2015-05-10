#[derive(RustcEncodable, RustcDecodable, Clone)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub url: String,
    pub order: Option<i32>,
}
