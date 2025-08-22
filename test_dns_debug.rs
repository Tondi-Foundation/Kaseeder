use std::net::UdpSocket;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    println!("=== DNS Debug Test ===");
    
    // Create a minimal DNS query for seed.kaspa.org A record
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

    println!("Query length: {} bytes", query.len());
    println!("Query hex: {:02x?}", query);
    
    // Create socket
    println!("Creating UDP socket...");
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(15)))?;
    
    let local_addr = socket.local_addr()?;
    println!("Socket bound to: {}", local_addr);
    
    // Send query
    println!("Sending DNS query to 127.0.0.1:8354...");
    let sent = socket.send_to(&query, "127.0.0.1:8354")?;
    println!("Sent {} bytes", sent);
    
    if sent != query.len() {
        println!("Warning: Sent {} bytes but query is {} bytes", sent, query.len());
    }
    
    println!("Query sent, waiting for response...");
    println!("(This may take up to 15 seconds)");
    
    // Receive response
    let mut buffer = [0u8; 1024];
    match socket.recv_from(&mut buffer) {
        Ok((len, addr)) => {
            println!("=== SUCCESS! ===");
            println!("Received {} bytes from {}", len, addr);
            println!("Response hex: {:02x?}", &buffer[..len]);
            
            // Try to parse as text
            if let Ok(text) = String::from_utf8(buffer[..len].to_vec()) {
                println!("Response as text: {}", text);
            }
            
            // Check if it looks like a DNS response
            if len >= 12 {
                let id = u16::from_be_bytes([buffer[0], buffer[1]]);
                let flags = u16::from_be_bytes([buffer[2], buffer[3]]);
                let questions = u16::from_be_bytes([buffer[4], buffer[5]]);
                let answers = u16::from_be_bytes([buffer[6], buffer[7]]);
                
                println!("DNS Response Analysis:");
                println!("  ID: 0x{:04x}", id);
                println!("  Flags: 0x{:04x}", flags);
                println!("  Questions: {}", questions);
                println!("  Answers: {}", answers);
                
                if flags & 0x8000 != 0 {
                    println!("  ✓ This is a DNS response");
                } else {
                    println!("  ✗ This doesn't look like a DNS response");
                }
            }
        }
        Err(e) => {
            println!("=== FAILED ===");
            println!("Error receiving response: {}", e);
            println!("Error kind: {:?}", e.kind());
            
            match e.kind() {
                std::io::ErrorKind::WouldBlock => println!("Socket timeout - no response received"),
                std::io::ErrorKind::TimedOut => println!("Socket timeout - no response received"),
                std::io::ErrorKind::ConnectionRefused => println!("Connection refused - DNS server not listening"),
                _ => println!("Other error occurred"),
            }
        }
    }
    
    Ok(())
}
