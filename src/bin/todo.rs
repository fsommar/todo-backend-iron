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

/*
fn reg<T: Encodable, H: TodoRepository<T>>(router: &mut Router, handler: H) {
    fn test<X: iron::Handler>(router: &mut Router, x: X) {
        // println!("test {:?}", handler);
        router.get("/todos/:id", x);
    }
    test(router, handler);
}
*/

fn main() {
    let mut router = Router::new();
    //router.get("/todos", get_todos);
    //router.get::<TodoRepository<&Todo>, &str>("/todos/:id", get_todo);
    //reg(&mut router, get_todo);
    router.get("/todos/:id", VecRepository(Box::new(get_todo)));
    router.get("/todos", VecRepository(Box::new(get_todos)));
    router.post("/todos", post_todo);
    router.delete("/todos", delete_todos);
    router.delete("/todos/:id", delete_todo);
    router.patch("/todos/:id", patch_todo);

    router.options("/todos", |_: &mut Request| Ok(Response::with(status::Ok)));
    router.options("/todos/:id", |_: &mut Request| Ok(Response::with(status::Ok)));

    let mut chain = Chain::new(router);
    chain.link_after(CorsFilter);
    chain.link(Write::<TodoList>::both(vec![]));

    Iron::new(chain).http("0.0.0.0:3000").unwrap();
}

struct TodoList;

impl Key for TodoList { type Value = Vec<Todo>; }

struct VecRepository<T>(Box<(Fn(&Vec<Todo>, &mut Request) -> Result<T, IronError> + Send + Sync)>);

impl<T: Encodable + Send + Sync + std::any::Any> iron::Handler for VecRepository<T> {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let mutex = req.get::<Write<TodoList>>().ok().unwrap();
        let list = mutex.lock().unwrap();
        let VecRepository(ref f) = *self;
        f(&*list, req).map(|v| Response::with((status::Ok, Json(&v))))
    }
}

fn get_todo(list: &Vec<Todo>, req: &mut Request) -> Result<Todo, IronError> {
    let id = req.extensions.get::<Router>().unwrap().find("id").unwrap();
    Ok(list.iter().find(|&x| x.id == id).unwrap().clone())
}

fn get_todos(list: &Vec<Todo>, req: &mut Request) -> Result<Vec<Todo>, IronError> {
    Ok(list.clone())
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
        url: format!("{}://{}:{}/todos/{}", req.url.scheme, req.url.host, req.url.port, id),
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

fn delete_todo(req: &mut Request) -> IronResult<Response> {
    let mutex = req.get::<Write<TodoList>>().ok().unwrap();
    let mut list = mutex.lock().unwrap();

    let id = req.extensions.get::<Router>().unwrap().find("id").unwrap();
    let pos = list.iter().position(|x| x.id == id).unwrap();
    list.swap_remove(pos);

    Ok(Response::with(status::Ok))
}

fn patch_todo(req: &mut Request) -> IronResult<Response> {
    let mutex = req.get::<Write<TodoList>>().ok().unwrap();
    let mut list = mutex.lock().unwrap();

    let todo = {
        let id = req.extensions.get::<Router>().unwrap().find("id").unwrap();
        list.iter_mut().find(|x| x.id == id).unwrap()
    };

    #[derive(RustcDecodable, Clone)]
    struct PatchTodo {
        title: Option<String>,
        completed: Option<bool>,
        order: Option<i32>,
    }

    let patch_todo = req.get::<bodyparser::Struct<PatchTodo>>().unwrap().unwrap();

    if let Some(title) = patch_todo.title {
        todo.title = title;
    }
    if let Some(completed) = patch_todo.completed {
        todo.completed = completed;
    }
    if let Some(order) = patch_todo.order {
        todo.order = Some(order);
    }

    Ok(Response::with((status::Ok, Json(todo.clone()))))
}

/// A simple wrapper struct for marking a struct as a JSON response.
struct Json<T: Encodable>(T);

impl<T: Encodable> iron::modifier::Modifier<Response> for Json<T> {
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
