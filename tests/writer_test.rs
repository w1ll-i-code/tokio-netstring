#[cfg(test)]
mod tests {
    use std::time::Duration;
    use tokio_netstring::NetstringWriter;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn should_write_netstring() {
        let msg = "Hello, World!";
        let expected = "13:Hello, World!,";

        let mut stream = Builder::new().write(expected.as_bytes()).build();

        stream
            .write_netstring(msg.as_bytes())
            .await
            .expect("Test passes");
    }

    #[tokio::test]
    async fn should_write_netstring_in_two_steps() {
        let msg = "Hello, World!";
        let expected = "13:Hello, World!,";
        let cut_off = 8;

        let mut stream = Builder::new()
            .write(&expected.as_bytes()[..cut_off])
            .wait(Duration::from_millis(5))
            .write(&expected.as_bytes()[cut_off..])
            .build();

        stream
            .write_netstring(msg.as_bytes())
            .await
            .expect("Test passes");
    }
}
