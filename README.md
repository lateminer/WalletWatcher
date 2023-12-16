WalletWatcher
===========================

A simple app for monitoring the last activity of specified crypto wallets. Initially designed for monitoring staking wallets.
Uses block explorer API to get data. At moment [Chainz](https://chainz.cryptoid.info/) and [BLNScan](https://blnexplorer.io/) are supported.

Contributions are welcome! To report issues or suggest enhancements, please create a new issue. If you want to contribute code, feel free to submit a pull request.

Setting up
-------

* Rename `coins-sample.toml` to `coins.toml`
* Set the addresses you would like to monitor in `coins.toml`

Usage
-------

* Run the app with `cargo run`
