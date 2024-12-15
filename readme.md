#### A chat server implemented in Rust programming language.

To run the server execute:
```cargo run --bin rust_pr```

To run a test with the user "dan" execute:
```cargo run --bin dan-test```

To run a test with the user "ian" execute:
```cargo run --bin ian-test```

Please note if at least one script is being run using Cargo the new calls will not trigger recompiling the code unless 
you stop all the actively running scripts.