# k8s-schedule-rs

## problem meet

```txt
Added: name="nginx-xx-xx", namespace="default"
Error create: SerdeError(Error("invalid value: string \"Status\", expected Pod", line: 1, column: 16))
Successfully assigned Pod nginx-xx-xx to Node xx
```

## fix problem with debug log

fix by change `Pod` to `Status`
with the help of debug log

```txt
2025-05-07T17:15:26.096211Z  WARN kube_client::client: {"kind":"Status","apiVersion":"v1","metadata":{},"status":"Success","code":201}
, Error("invalid value: string \"Status\", expected Pod", line: 1, column: 16)
```

## run

```bash
cargo run
```
