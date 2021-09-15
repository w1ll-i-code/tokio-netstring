#[cfg(test)]
mod tests {
    use tokio::time::Duration;
    use tokio_netstring::NetstringReader;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn should_parse_netstring() {
        let msg = "13:Hello, World!,";
        let expected = "Hello, World!";
        let mut buf = [0; 13];

        let mut test = Builder::new().read(msg.as_bytes()).build();

        test.read_netstring(&mut buf)
            .await
            .expect("Test should pass");

        assert_eq!(expected.as_bytes(), buf);
    }

    #[tokio::test]
    async fn should_parse_netstring_in_two_steps() {
        let msg = "13:Hello, World!,";
        let expected = "Hello, World!";
        let split = 10;
        let mut buf = [0; 13];

        let mut test = Builder::new()
            .read(&msg.as_bytes()[..split])
            .wait(Duration::from_micros(5))
            .read(&msg.as_bytes()[split..])
            .build();

        test.read_netstring(&mut buf)
            .await
            .expect("Test should pass");

        assert_eq!(expected.as_bytes(), buf);
    }

    #[tokio::test]
    async fn should_fail_on_incomplete_message() {
        let msg = "13:Hello, World!,";
        let split = 10;
        let mut buf = [0; 13];

        let mut test = Builder::new().read(&msg.as_bytes()[..split]).build();

        test.read_netstring(&mut buf)
            .await
            .expect_err("Message not finished");
    }

    #[tokio::test]
    async fn should_fail_on_incomplete_message_missing_terminator() {
        let msg = "13:Hello, World!";
        let split = 10;
        let mut buf = [0; 13];

        let mut test = Builder::new().read(&msg.as_bytes()[..split]).build();

        test.read_netstring(&mut buf)
            .await
            .expect_err("Message not finished");
    }
}
