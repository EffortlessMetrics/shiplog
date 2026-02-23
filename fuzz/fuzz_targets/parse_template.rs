//! Fuzz harness for template files
//!
//! This harness tests robustness of template parser against
//! malformed or unexpected template syntax.
//! Target: Jinja2-like template syntax

#![no_main]

use libfuzzer_sys::fuzz_target;

// Simple template parser that handles Jinja2-like syntax
fn parse_template(text: &str) -> Result<(), String> {
    let mut depth = 0;
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    
    while i < len {
        // Check for opening block
        if i + 1 < len && chars[i] == '{' && chars[i + 1] == '%' {
            depth += 1;
            i += 2;
            continue;
        }
        
        // Check for closing block
        if i + 1 < len && chars[i] == '%' && chars[i + 1] == '}' {
            if depth == 0 {
                return Err("Unmatched closing block".to_string());
            }
            depth -= 1;
            i += 2;
            continue;
        }
        
        // Check for opening variable
        if i + 1 < len && chars[i] == '{' && chars[i + 1] == '{' {
            i += 2;
            continue;
        }
        
        // Check for closing variable
        if i + 1 < len && chars[i] == '}' && chars[i + 1] == '}' {
            i += 2;
            continue;
        }
        
        i += 1;
    }
    
    if depth != 0 {
        return Err("Unclosed block".to_string());
    }
    
    Ok(())
}

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, ignore invalid UTF-8 (handled error)
    if let Ok(text) = std::str::from_utf8(data) {
        // Try to parse the template
        let _ = parse_template(text);
    }
});
