# HTTP Server Example

## Basic Server
```io
import { Server } from "http";

fn main() {
    let server = Server::new();
    
    server.route("/", fn(req, res) {
        res.send("Hello, Web!");
    });
    
    server.route("/api", fn(req, res) {
        res.json({
            status: "ok",
            message: "API working"
        });
    });
    
    server.listen(8080);
}
```

## Middleware Example
```io
fn logger(req, res, next) {
    println("[" + new_date() + "] " + req.method + " " + req.path);
    next();
}

server.use(logger);
```

## Error Handling
```io
server.error(fn(err, req, res) {
    res.status(500).json({
        error: err.message
    });
});
```
