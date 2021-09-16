#[cfg(test)]
mod tests {
    use tokio::time::Duration;
    use tokio_netstring::NetstringReader;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn should_drop_netstring() {
        let msg = "13:Hello, World!,";

        let mut test = Builder::new().read(msg.as_bytes()).build();

        test.drop_netstring().await.expect("Test should pass");
    }

    #[tokio::test]
    async fn should_drop_netstring_in_two_steps() {
        let msg = "13:Hello, World!,";
        let split = 10;

        let mut test = Builder::new()
            .read(&msg.as_bytes()[..split])
            .wait(Duration::from_micros(5))
            .read(&msg.as_bytes()[split..])
            .build();

        test.drop_netstring().await.expect("Test should pass");
    }
    #[tokio::test]
    async fn should_drop_netstring_byte_by_byte() {
        let msg = "13:Hello, World!,";
        let mut test = Builder::new();

        for i in 0..msg.len() {
            test.read((&msg[i..i + 1]).as_bytes())
                .wait(Duration::from_micros(5));
        }

        test.build().drop_netstring().await.expect("Test should pass");
    }
}
