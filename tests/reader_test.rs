#[cfg(test)]
mod tests {
    use tokio::time::Duration;
    use tokio_test::io::Builder;
    use tokio_netstring::NetstringReader;

    #[test]
    fn should_parse_netstring() {
        let msg = "13:Hello, World!,";
        let expected = "Hello, World!";
        let mut buf = [0; 13];

        let mut test = Builder::new().read(msg.as_bytes()).build();

        tokio_test::block_on(test.read_netstring(&mut buf)).expect("Test should pass");

        assert_eq!(expected.as_bytes(), buf);
    }

    #[test]
    fn should_parse_netstring_in_two_steps() {
        let msg = "13:Hello, World!,";
        let expected = "Hello, World!";
        let split = 10;
        let mut buf = [0; 13];

        let mut test = Builder::new()
            .read(&msg.as_bytes()[..split])
            .wait(Duration::from_micros(5))
            .read(&msg.as_bytes()[split..])
            .build();

        tokio_test::block_on(test.read_netstring(&mut buf)).expect("Test should pass");

        assert_eq!(expected.as_bytes(), buf);
    }

    #[test]
    fn should_fail_on_incomplete_message() {
        let msg = "13:Hello, World!,";
        let split = 10;
        let mut buf = [0; 13];

        let mut test = Builder::new().read(&msg.as_bytes()[..split]).build();

        tokio_test::block_on(test.read_netstring(&mut buf)).expect_err("Message not finished");
    }

    #[test]
    fn should_fail_on_incomplete_message_missing_terminator() {
        let msg = "13:Hello, World!";
        let split = 10;
        let mut buf = [0; 13];

        let mut test = Builder::new().read(&msg.as_bytes()[..split]).build();

        tokio_test::block_on(test.read_netstring(&mut buf)).expect_err("Message not finished");
    }
}