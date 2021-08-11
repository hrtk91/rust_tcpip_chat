use std::net::TcpListener;
use std::io::{ Read, Write };

const HOST: &str = "[::]";
const PORT: &str = "8080";

fn main() {
    println!("program start");

    let listener = TcpListener::bind(format!("{}:{}", HOST, PORT));
    if let Err(e) = listener {
        println!("fault create listener: {:?}", e);
        return
    }

    let tcp_listener = listener.unwrap();

    loop {
        println!("listen start");
        let accept_result = tcp_listener.accept();

        if let Err(e) = accept_result {
            println!("fault listener accept start: {:?}", e);
            return
        }

        let (mut stream, addr) = accept_result.unwrap();

        println!("connect to {}", addr);

        let mut buf = [ 0u8; 1024 ];

        loop {
            let size =  stream.read(&mut buf);
            println!("stream reading");

            if let Err(e) = size {
                println!("fault stream read: {:?}", e);
                return
            }
            
            let size = size.unwrap();

            if size == 0 {
                println!("connection closed");
                return
            }

            let res = String::from_utf8(buf[..size].to_vec());

            if let Err(e) = res {
                println!("fault convert response: {:?}", e);
                return
            }

            let res = res.unwrap();
            println!("received: {}", res); 

            if let Err(e) = stream.write(&buf[..size]) {
                println!("echo failed:{:?}", e);
                return
            }
        }
    }
}

