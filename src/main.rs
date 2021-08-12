use std::net::TcpStream;
use std::sync::mpsc;

mod ruscha;

fn rewrite(queue: &std::collections::VecDeque<ruscha::ThreadMessage>) {
    for thread_msg in queue.iter() {
        match thread_msg.category {
            ruscha::Category::Send => {
                println!("me: {}", thread_msg.content);
            },
            ruscha::Category::Receive => {
                println!("partner: {}", thread_msg.content);
            },
            _ => { }
        };
    }
}

fn main_loop(rx: mpsc::Receiver<ruscha::ThreadMessage>) {
    use std::collections::VecDeque;
    let mut queue: VecDeque<ruscha::ThreadMessage> = std::collections::VecDeque::new();
    println!("{}", termion::clear::All);
    loop {
        let thread_msg = match rx.recv() {
            Ok(msg) => match msg.category {
                ruscha::Category::Terminate => {
                    println!("connection terminated");
                    println!("program shutdown");
                    return
                },
                _ => msg,
            },
            Err(e) => panic!("cannot receive thread message: {:?}", e),
        };

        queue.push_back(thread_msg);
        if queue.len() > 10 {
            queue.pop_front();
        };
        println!("{}", termion::clear::All);
        rewrite(&queue);
    }
}

fn main() {
    println!("program start");

    let input = loop {
        println!("connect or accept:");
        let mut input = String::new();
        let result = std::io::stdin().read_line(&mut input);
        if let Ok(_) = result {
            break input.trim().to_string();
        } else {
            continue;
        }
    };

    let stream: TcpStream = match input.as_str() {
        "connect" => {
            println!("host:");
            let mut input = String::new();
            let host = match std::io::stdin().read_line(&mut input) {
                Ok(_) => input.trim().to_string(),
                Err(e) => panic!("input failed: {:?}", e),
            };

            println!("port:");
            let mut input = String::new();
            let port = match std::io::stdin().read_line(&mut input) {
                Ok(_) => input.trim().to_string(),
                Err(e) => panic!("input failed: {:?}", e),
            };

            ruscha::connect(host, port).unwrap()
        },
        "accept" => {
            ruscha::listen(ruscha::HOST.to_string(), ruscha::PORT).unwrap().0
        },
        _ => return println!("input error"),
    };

    let (rx, _receive_handle, _send_handle) = ruscha::start(stream);

    main_loop(rx);
}
