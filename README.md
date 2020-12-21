# ramen

Low-level windowing and video initialization for real-time graphical applications, especially video games.

## Basic Example

```rust
use ramen::{event::Event, monitor::Size, window::Window};

// Create your window
let mut window = Window::builder()
    .inner_size(Size::Logical(1280.0, 720.0))
    .resizable(false)
    .title("a nice window")
    .build()?;


// Poll events & do your processing
'main: loop {
    for event in window.events() {
        match event {
            Event::Close(_) => break 'main,
            _ => (),
        }
    }

    // Render graphics, process input, et cetera.

    window.swap_events();
}
```
