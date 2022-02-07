rm tf2-bot-kicker-gui
rm tf2-bot-kicker-gui.exe

rm target/release/tf2-bot-kicker-gui
rm target/x86_64-pc-windows-gnu/release/tf2-bot-kicker-gui.exe

cargo build --release 
cp target/release/tf2-bot-kicker-gui .

cargo build --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/tf2-bot-kicker-gui.exe .
