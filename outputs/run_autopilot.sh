#!/bin/sh
# lengsuo-desktop 自动驾驶入口：拉起常驻守护（守护自己会每小时跑一轮）
#
# 用途：47 研究机上让界面线自动推进 —— 每小时合并门禁全绿的卡、按 lane 派新卡。
# 47 没有 cron 守护，且 systemd unit 需要 root，所以用 nohup 常驻 + 自愈循环。
#
# 手动跑一轮（不等定时）：ssh 47 'cd ~/lengsuo-desktop-dev && python3 autopilot.py'
# 看状态：              ssh 47 'cd ~/lengsuo-desktop-dev && python3 autopilot.py --status'
# 看日志：              ssh 47 'tail -50 ~/lengsuo-desktop-dev/logs/autopilot-daemon.out'
#
# 注意：不能用 `pgrep -f autopilot_daemon.py` 判断是否已在运行 ——
# 它会匹配到调用它的那条 shell 命令行自身（历史踩坑）。改用 ps + 精确正则。
export PATH=/home/ubuntu/.cargo/bin:/home/ubuntu/.local/bin:/usr/local/bin:/usr/bin:/bin
cd /home/ubuntu/lengsuo-desktop-dev || exit 1

if ps -eo pid,args | grep -E "^ *[0-9]+ python3 .*autopilot_daemon\.py" >/dev/null 2>&1; then
  echo "守护已在运行"
  exit 0
fi

mkdir -p logs
setsid nohup python3 autopilot_daemon.py --interval 3600 >> logs/autopilot-daemon.out 2>&1 < /dev/null &
sleep 2
echo "守护已启动"
