use std::net::UdpSocket;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    // Create a simple DNS query for seed.kaspa.org A record
    // This is a minimal valid DNS query
    let query = [
        0x12, 0x34, // ID (random)
        0x01, 0x00, // Flags: standard query
        0x00, 0x01, // Questions: 1
        0x00, 0x00, // Answer RRs: 0
        0x00, 0x00, // Authority RRs: 0
        0x00, 0x00, // Additional RRs: 0
        // Question section: seed.kaspa.org
        0x04, 0x73, 0x65, 0x65, 0x64, // "seed" (4 bytes)
        0x05, 0x6b, 0x61, 0x73, 0x70, 0x61, // "kaspa" (5 bytes)
        0x03, 0x6f, 0x72, 0x67, // "org" (3 bytes)
        0x00, // End of name
        0x00, 0x01, // Type: A (1)
        0x00, 0x01, // Class: IN (1)
    ];

    println!("Sending DNS query to 127.0.0.1:5354");
    println!("Query length: {} bytes", query.len());
    println!("Query hex: {:02x?}", query);
    
    // Create socket
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(10)))?;
    
    // Send query
    let sent = socket.send_to(&query, "127.0.0.1:5354")?;
    println!("Sent {} bytes", sent);
    println!("Query sent, waiting for response...");
    
    // Receive response
    let mut buffer = [0u8; 1024];
    match socket.recv_from(&mut buffer) {
        Ok((len, addr)) => {
            println!("Received {} bytes from {}", len, addr);
            println!("Response hex: {:02x?}", &buffer[..len]);
            
            // Try to parse as text
            if let Ok(text) = String::from_utf8(buffer[..len].to_vec()) {
                println!("Response as text: {}", text);
            }
        }
        Err(e) => {
            println!("Error receiving response: {}", e);
        }
    }
    
    Ok(())
}
