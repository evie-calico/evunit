# build gb image if source is newer
if [ "test/test.asm" -nt "test/test.gb" ]; then
    rgbasm -o - test/test.asm | rgblink -o test/test.gb -n test/test.sym -
fi

# run evunit
cargo run -- -c test/test.toml -d dump -n test/test.sym test/test.gb
