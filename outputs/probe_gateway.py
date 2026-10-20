"""直连模型网关做一次最小 messages 调用，判定 403 是否仍存在。

用法: python3 probe_gateway.py [model]
退出码: 0 = 网关可用; 1 = 网关仍不可用
"""
import json
import sys
import urllib.error
import urllib.request

GATEWAY = "http://192.168.1.151:18080"
MODEL = sys.argv[1] if len(sys.argv) > 1 else "auto"

body = {
    "model": MODEL,
    "max_tokens": 64,
    "messages": [{"role": "user", "content": "reply with the single word: ok"}],
}
req = urllib.request.Request(
    f"{GATEWAY}/v1/messages",
    data=json.dumps(body).encode(),
    headers={
        "content-type": "application/json",
        "anthropic-version": "2023-06-01",
        "authorization": "Bearer local",
        "x-api-key": "local",
    },
    method="POST",
)

print(f"POST {GATEWAY}/v1/messages  model={MODEL}")
try:
    with urllib.request.urlopen(req, timeout=120) as resp:
        raw = resp.read()
        print("HTTP", resp.status, "| bytes:", len(raw))
        print("content-type:", resp.headers.get("content-type"))
        text = raw.decode("utf-8", "replace")
        print("body[:600]:", text[:600])
        if resp.status == 200 and text.strip():
            print("RESULT: 网关可用")
            sys.exit(0)
        print("RESULT: 200 但响应体异常")
        sys.exit(1)
except urllib.error.HTTPError as exc:
    detail = exc.read().decode("utf-8", "replace")[:600]
    print("HTTP", exc.code)
    print("body:", detail)
    print("RESULT: 网关不可用")
    sys.exit(1)
except Exception as exc:  # noqa: BLE001
    print("异常:", type(exc).__name__, exc)
    print("RESULT: 连接失败")
    sys.exit(1)
