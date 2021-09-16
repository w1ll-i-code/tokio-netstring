#[cfg(test)]
mod tests {
    use std::time::Duration;
    use tokio_netstring_trait::NetstringWriter;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn should_write_netstring() {
        let msg = "Hello, World!";
        let expected = "13:Hello, World!,";

        let mut test = Builder::new().write(expected.as_bytes()).build();

        test.write_netstring(msg.as_bytes())
            .await
            .expect("Test passes");
    }

    #[tokio::test]
    async fn should_write_netstring_in_two_steps() {
        let msg = "Hello, World!";
        let expected = "13:Hello, World!,";
        let cut_off = 8;

        let mut test = Builder::new()
            .write(&expected.as_bytes()[..cut_off])
            .wait(Duration::from_millis(5))
            .write(&expected.as_bytes()[cut_off..])
            .build();

        test.write_netstring(msg.as_bytes())
            .await
            .expect("Test passes");
    }

    #[tokio::test]
    async fn should_write_netstring_byte_by_byte() {
        let msg = "Hello, World!";
        let expected = "13:Hello, World!,";

        let mut test = Builder::new();

        for i in 0..expected.len() {
            test.write(&expected.as_bytes()[i..i+1])
                .wait(Duration::from_millis(5));
        }

        test.build()
            .write_netstring(msg.as_bytes())
            .await
            .expect("Test passes");
    }

    #[tokio::test]
    async fn should_write_zero_length_netstring_byte_by_byte() {
        let msg = "";
        let expected = "0:,";

        let mut test = Builder::new();

        for i in 0..expected.len() {
            test.write(&expected.as_bytes()[i..i+1])
                .wait(Duration::from_millis(5));
        }

        test.build()
            .write_netstring(msg.as_bytes())
            .await
            .expect("Test passes");
    }

}
