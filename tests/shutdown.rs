use {
  super::*,
  nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
  },
  std::time::Instant,
};

struct ServerProcess(Child);

impl Drop for ServerProcess {
  fn drop(&mut self) {
    if self.0.try_wait().unwrap().is_none() {
      let _ = self.0.kill();
      let _ = self.0.wait();
    }
  }
}

fn start(core: &mockcore::Handle, dir: Arc<TempDir>) -> (ServerProcess, String) {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let port = listener.local_addr().unwrap().port();
  drop(listener);
  // Production mode uses durable commits and the real polling interval.
  let child = CommandBuilder::new(format!(
    "--index-cache-size 33554432 --commit-interval 100 --index-sats --index-runes --index-addresses server --address 127.0.0.1 --http-port {port} --polling-interval 2s"
  ))
  .integration_test(false)
  .core(core)
  .temp_dir(dir)
  .command()
  .spawn()
  .unwrap();
  (ServerProcess(child), format!("http://127.0.0.1:{port}"))
}

fn wait_for_blocks(server: &mut ServerProcess, url: &str, count: u64) {
  let client = reqwest::blocking::Client::builder()
    .timeout(Duration::from_secs(1))
    .build()
    .unwrap();
  let deadline = Instant::now() + Duration::from_secs(30);
  let mut last = String::new();
  loop {
    assert!(
      server.0.try_wait().unwrap().is_none(),
      "server exited before indexing"
    );
    if let Ok(response) = client.get(format!("{url}/blockcount")).send() {
      let status = response.status();
      last = response.text().unwrap();
      if status.is_success() && last.trim().parse::<u64>() == Ok(count) {
        assert_eq!(
          client
            .get(format!("{url}/healthz"))
            .send()
            .unwrap()
            .status(),
          StatusCode::OK
        );
        assert_eq!(
          client.get(format!("{url}/readyz")).send().unwrap().status(),
          StatusCode::OK
        );
        return;
      }
    }
    if Instant::now() >= deadline {
      server.0.kill().unwrap();
      server.0.wait().unwrap();
      let mut stderr = String::new();
      use std::io::Read;
      server
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
      panic!("index did not reach {count} blocks, last response {last}: {stderr}");
    }
    thread::sleep(Duration::from_millis(25));
  }
}

fn stop(mut server: ServerProcess, repeat: bool) {
  let pid = Pid::from_raw(server.0.id().try_into().unwrap());
  kill(pid, Signal::SIGTERM).unwrap();
  if repeat {
    // The indexer is asleep while HTTP drains; another signal must not bypass
    // the main thread's join or redb's Drop.
    thread::sleep(Duration::from_millis(50));
    kill(pid, Signal::SIGTERM).unwrap();
    kill(pid, Signal::SIGINT).unwrap();
  }
  let deadline = Instant::now() + Duration::from_secs(15);
  let status = loop {
    if let Some(status) = server.0.try_wait().unwrap() {
      break status;
    }
    assert!(Instant::now() < deadline, "graceful shutdown timed out");
    thread::sleep(Duration::from_millis(25));
  };
  let mut stderr = String::new();
  use std::io::Read;
  server
    .0
    .stderr
    .take()
    .unwrap()
    .read_to_string(&mut stderr)
    .unwrap();
  assert!(status.success(), "unclean exit: {status}\n{stderr}");
}

fn assert_clean_index(core: &mockcore::Handle, dir: Arc<TempDir>, minimum: u64) {
  // Opening redb in production mode exercises its recovery check. The info
  // command also validates tables and updates any remaining blocks.
  let output = CommandBuilder::new("--index-cache-size 33554432 index info")
    .integration_test(false)
    .core(core)
    .temp_dir(dir)
    .command()
    .output()
    .unwrap();
  let stderr = String::from_utf8(output.stderr).unwrap();
  assert!(output.status.success(), "{stderr}");
  assert!(
    !stderr.contains("needs recovery")
      && !String::from_utf8_lossy(&output.stdout).contains("needs recovery"),
    "redb was not cleanly closed: {stderr}"
  );
  let info: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
  assert!(info["blocks_indexed"].as_u64().unwrap() >= minimum);
}

fn mine(core: &mockcore::Handle, count: u64) {
  // Bells has a height-dependent subsidy; Bitcoin's mock default (50 coins)
  // exceeds the valid reward in the first 100 Bells blocks.
  for _ in 0..count {
    let height = ordinals::Height((core.height() + 1).try_into().unwrap());
    core.mine_blocks_with_subsidy(1, height.subsidy(Network::Bellscoin));
  }
}

#[test]
fn indexed_state_survives_sigterm_and_repeated_signals() {
  let core = mockcore::spawn();
  mine(&core, 12);
  let dir = Arc::new(TempDir::new().unwrap());
  for repeat in [false, true] {
    let (mut server, url) = start(&core, dir.clone());
    wait_for_blocks(&mut server, &url, 13);
    thread::sleep(Duration::from_millis(150));
    stop(server, repeat);
    assert_clean_index(&core, dir.clone(), 13);
  }
  mine(&core, 3);
  let (mut server, url) = start(&core, dir.clone());
  wait_for_blocks(&mut server, &url, 16);
  stop(server, false);
  assert_clean_index(&core, dir, 16);
}

#[test]
fn shutdown_during_indexing_commits_and_can_resume() {
  let core = mockcore::spawn();
  mine(&core, 12);
  let dir = Arc::new(TempDir::new().unwrap());
  let (mut server, url) = start(&core, dir.clone());
  wait_for_blocks(&mut server, &url, 13);
  // A batch exceeds the commit interval and leaves unfinished work at TERM.
  mine(&core, 3000);
  let client = reqwest::blocking::Client::new();
  let deadline = Instant::now() + Duration::from_secs(30);
  loop {
    let count: u64 = client
      .get(format!("{url}/blockcount"))
      .send()
      .unwrap()
      .text()
      .unwrap()
      .trim()
      .parse()
      .unwrap();
    assert!(
      count < 3013,
      "fixture completed before an active batch was observed"
    );
    if count > 13 {
      break;
    }
    assert!(Instant::now() < deadline, "new batch never started");
    thread::sleep(Duration::from_millis(2));
  }
  stop(server, false);
  assert_clean_index(&core, dir.clone(), 3013);
  let (mut server, url) = start(&core, dir.clone());
  wait_for_blocks(&mut server, &url, 3013);
  stop(server, true);
  assert_clean_index(&core, dir, 3013);
}
