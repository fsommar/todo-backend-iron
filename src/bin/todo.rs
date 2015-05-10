extern crate todo_iron as todo;
extern crate iron;
extern crate router;
extern crate uuid;
extern crate bodyparser;
extern crate persistent;
extern crate rustc_serialize;
extern crate unicase;

use todo::*;

use iron::prelude::*;
use iron::{status, headers};
use iron::method::Method::*;
use persistent::Write;
use iron::typemap::Key;

use router::Router;

use uuid::Uuid;

use unicase::UniCase;

use rustc_serialize::{json, Encodable};

fn main() {
    let mut router = Router::new();
    router.get("/todos", get_todos);
    router.get("/todos/:id", get_todo);
    router.post("/todos", post_todo);
    router.delete("/todos", delete_todos);
    router.options("/todos", |_: &mut Request| Ok(Response::with(status::Ok)));

    let mut chain = Chain::new(router);
    chain.link_after(CorsFilter);
    chain.link(Write::<TodoList>::both(vec![]));

    Iron::new(chain).http("0.0.0.0:3000").unwrap();
}

struct TodoList;

impl Key for TodoList { type Value = Vec<Todo>; }

fn get_todo(req: &mut Request) -> IronResult<Response> {
    let mutex = req.get::<Write<TodoList>>().ok().unwrap();
    let list = mutex.lock().unwrap();

    let id = req.extensions.get::<Router>().unwrap().find("id").unwrap();
    let todo = list.iter().find(|&x| x.id == id).unwrap();

    Ok(Response::with((status::Ok, Json(&todo))))
}

fn get_todos(req: &mut Request) -> IronResult<Response> {
    let mutex = req.get::<Write<TodoList>>().ok().unwrap();
    let list = mutex.lock().unwrap();

    Ok(Response::with((status::Ok, Json(&*list))))
}

fn post_todo(req: &mut Request) -> IronResult<Response> {
    let mutex = req.get::<Write<TodoList>>().ok().unwrap();
    let mut list = mutex.lock().unwrap();

    #[derive(RustcDecodable, Clone)]
    struct PostTodo {
        title: String,
        order: Option<i32>,
    }

    let post_todo = req.get::<bodyparser::Struct<PostTodo>>().unwrap().unwrap();

    let id = Uuid::new_v4().to_string();

    let todo = Todo {
        id: id.clone(),
        title: post_todo.title,
        order: post_todo.order,
        completed: false,
        url: format!("/todos/{}", id),
    };

    list.push(todo.clone());

    Ok(Response::with((status::Ok, Json(&todo))))
}

fn delete_todos(req: &mut Request) -> IronResult<Response> {
    let mutex = req.get::<Write<TodoList>>().ok().unwrap();
    let mut list = mutex.lock().unwrap();

    list.clear();

    Ok(Response::with(status::Ok))
}

/// A simple wrapper struct for marking a struct as a JSON response.
struct Json<'a, T: Encodable + 'a>(&'a T);

impl<'a, T: Encodable> iron::modifier::Modifier<Response> for Json<'a, T> {
    #[inline]
    fn modify(self, res: &mut Response) {
        let Json(x) = self;
        // Make sure the content type is marked as JSON
        res.headers.set(headers::ContentType("application/json".parse().unwrap()));
        res.set_mut(json::encode(&x).unwrap());
    }
}

struct CorsFilter;

impl iron::AfterMiddleware for CorsFilter {
    fn after(&self, _: &mut Request, mut res: Response) -> IronResult<Response> {
        res.headers.set(headers::AccessControlAllowOrigin::Any);
        res.headers.set(headers::AccessControlAllowHeaders(
                vec![UniCase("accept".to_string()),
                UniCase("content-type".to_string())]));
        res.headers.set(headers::AccessControlAllowMethods(
                vec![Get,Head,Post,Delete,Options,Put,Patch]));
        Ok(res)
    }
}
