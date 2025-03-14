#!/bin/bash
# Script to gather system resource information for WTF dashboard

# CPU usage
CPU_USAGE=$(top -bn1 | grep "Cpu(s)" | sed "s/.*, *\([0-9.]*\)%* id.*/\1/" | awk '{print 100 - $1"%"}')

# Memory usage
MEM_TOTAL=$(free -m | awk '/Mem:/ {print $2}')
MEM_USED=$(free -m | awk '/Mem:/ {print $3}')
MEM_PERCENT=$(free | grep Mem | awk '{print $3/$2 * 100.0}' | cut -d. -f1)

# Disk usage
DISK_USAGE=$(df -h / | awk 'NR==2 {print $5}')
DISK_AVAIL=$(df -h / | awk 'NR==2 {print $4}')

# Network activity - instantaneous traffic (bytes per second)
# First measurement
NETWORK_RX_1=$(cat /proc/net/dev | grep -v "lo" | awk 'NR>2 {sum += $2} END {print sum}')
NETWORK_TX_1=$(cat /proc/net/dev | grep -v "lo" | awk 'NR>2 {sum += $10} END {print sum}')

# Wait for a second
sleep 1

# Second measurement
NETWORK_RX_2=$(cat /proc/net/dev | grep -v "lo" | awk 'NR>2 {sum += $2} END {print sum}')
NETWORK_TX_2=$(cat /proc/net/dev | grep -v "lo" | awk 'NR>2 {sum += $10} END {print sum}')

# Calculate difference (bytes per second)
NETWORK_RX_SPEED=$((NETWORK_RX_2 - NETWORK_RX_1))
NETWORK_TX_SPEED=$((NETWORK_TX_2 - NETWORK_TX_1))

# Convert bytes/sec to human-readable format
format_bytes_per_second() {
  local bytes=$1
  if [ $bytes -gt 1048576 ]; then
    echo $(echo "scale=2; $bytes/1048576" | bc)" MB/s"
  elif [ $bytes -gt 1024 ]; then
    echo $(echo "scale=2; $bytes/1024" | bc)" KB/s"
  else
    echo "$bytes B/s"
  fi
}

NETWORK_RX_FORMATTED=$(format_bytes_per_second $NETWORK_RX_SPEED)
NETWORK_TX_FORMATTED=$(format_bytes_per_second $NETWORK_TX_SPEED)

# Output formatted for dashboard
printf "CPU Usage    : %s\n" "$CPU_USAGE"
printf "Memory       : %s MB / %s MB (%s%%)\n" "$MEM_USED" "$MEM_TOTAL" "$MEM_PERCENT"
printf "Disk Space   : %s used (%s available)\n" "$DISK_USAGE" "$DISK_AVAIL"
printf "Network      : ↓ %s / ↑ %s\n" "$NETWORK_RX_FORMATTED" "$NETWORK_TX_FORMATTED"