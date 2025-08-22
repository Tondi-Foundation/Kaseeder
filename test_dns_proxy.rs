use std::net::TcpStream;
use std::io::{Read, Write};
use std::time::Duration;

fn main() -> std::io::Result<()> {
    // Connect to VPN proxy
    let mut stream = TcpStream::connect("127.0.0.1:7897")?;
    stream.set_read_timeout(Some(Duration::from_secs(10))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    
    println!("Connected to VPN proxy at 127.0.0.1:7897");
    
    // Create a simple HTTP CONNECT request to establish tunnel
    let connect_request = format!(
        "CONNECT 127.0.0.1:5354 HTTP/1.1\r\n\
         Host: 127.0.0.1:5354\r\n\
         User-Agent: DNS-Test/1.0\r\n\
         \r\n"
    );
    
    println!("Sending CONNECT request...");
    stream.write_all(connect_request.as_bytes())?;
    
    // Read response
    let mut response = [0u8; 1024];
    let len = stream.read(&mut response)?;
    let response_text = String::from_utf8_lossy(&response[..len]);
    println!("Proxy response: {}", response_text);
    
    if response_text.contains("200") {
        println!("Tunnel established, sending DNS query...");
        
        // Create DNS query for seed.kaspa.org A record
        let dns_query = [
            0x12, 0x34, // ID
            0x01, 0x00, // Flags: standard query
            0x00, 0x01, // Questions: 1
            0x00, 0x00, // Answer RRs: 0
            0x00, 0x00, // Authority RRs: 0
            0x00, 0x00, // Additional RRs: 0
            // Question section: seed.kaspa.org
            0x04, 0x73, 0x65, 0x65, 0x64, // "seed"
            0x05, 0x6b, 0x61, 0x73, 0x70, 0x61, // "kaspa"
            0x03, 0x6f, 0x72, 0x67, // "org"
            0x00, // End of name
            0x00, 0x01, // Type: A
            0x00, 0x01, // Class: IN
        ];
        
        // Send DNS query through tunnel
        stream.write_all(&dns_query)?;
        println!("DNS query sent through tunnel");
        
        // Read DNS response
        let mut dns_response = [0u8; 1024];
        match stream.read(&mut dns_response) {
            Ok(len) => {
                println!("Received {} bytes DNS response", len);
                println!("Response hex: {:02x?}", &dns_response[..len]);
            }
            Err(e) => {
                println!("Error reading DNS response: {}", e);
            }
        }
    } else {
        println!("Failed to establish tunnel");
    }
    
    Ok(())
}
