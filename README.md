# port-tester
Port tester (pt) provides repetative port tests and returns a summary, similar to ping.
For use in detecting connectivity issues to a specific service.

## Help
```
Usage: pt [OPTIONS] <HOST> [PORT]

Arguments:
  <HOST>  Target host to connect to
  [PORT]  Port number to connect to [default: 443]

Options:
  -c, --count <COUNT>
          Count of connection attempts. Use 0 for infinite attempts [default: 0]
  -i, --interval <INTERVAL>
          Interval between attempts in seconds [default: 1]
  -q, --quiet
          Quiet mode. Suppress per-attempt output and attempt errors only showing sequence numbers and each result as 'ok' or 'failed'
  -r, --report-interval <REPORT_INTERVAL>
          Interval to output intermediate reports. Default is 0 (no intermediate reports). If set to N, a report will be printed every N attempts [default: 0]
  -s, --silent
          Silent mode. Suppress output except for errors and final report
  -t, --timeout <TIMEOUT>
          Connection attempt timeout in seconds [default: 5]
  -h, --help
          Print help
  -V, --version
          Print version
```

## Examples
By defautl pt makes an infinite number of attempts and must be interrupted. pt will try to print a report on interrupt.

Connect to 8.8.8.8 on port 53 every 5 secs. Make 10 attempts.
```
❯ pt 8.8.8.8 53 -c 10
Connecting to 8.8.8.8 (8.8.8.8) on 53
1 ok
2 ok
3 ok
4 ok
5 ok
6 ok
7 ok
8 ok
9 ok
10 ok
10 attempts, success: 10, fail: 0, failure rate: 0.00%
```

Provide an intermediate report ever 5 attempts.
```
❯ pt -c 10 -r 5 -i 5 8.8.8.8 53
Connecting to 8.8.8.8 (8.8.8.8) on 53
1 ok
2 ok
3 ok
4 ok
5 ok
Intermediate report: 5 attempts, success: 5, fail: 0, failure rate: 0.00%
6 ok
7 ok
8 ok
9 ok
10 ok
10 attempts, success: 10, fail: 0, failure rate: 0.00%
```

A connection count of 1 will only output the single attempt summary with no report afterwards. You can get one word results by using quit (-q).
```
❯ pt 8.8.8.8 53 -c 1
Connecting to 8.8.8.8 (8.8.8.8) on 53
ok
❯ pt -c 1 -q 8.8.8.8 53
ok
```

For single attempts with silent (-s), pt will return a success (0) or error (1) code and no other output. This can be useful for performing other actions depending on a simple sucess/fail code.
```
❯ pt -c 1 -s 8.8.8.8 53; echo $?
0
❯ pt -c 1 -s 8.8.8.8 80; echo $?
1
```