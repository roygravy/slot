#!/bin/sh
SD="${SLOT_ROOT:-/mnt/sdcard}"
SYS="$SD/System"
RUN="${AGS_RUN:-/run}"
LOGS="$SD/Logs"
SESSION_MAX=65536
export SLOT_ROOT="$SD"
export SLOT_RUN="$RUN"

mkdir -p "$LOGS" 2>/dev/null
for f in slot.log slot.log.1 slot-session.log slot-session.log.1 slot-card.log ags-shutdown-trace.log input-trace.log; do
	[ -f "$SD/$f" ] && mv -f "$SD/$f" "$LOGS/$f"
done
[ -d "$SD/crash" ] && [ ! -e "$LOGS/crash" ] && mv "$SD/crash" "$LOGS/crash"

[ $(wc -c < "$LOGS/slot-session.log" 2>/dev/null || echo 0) -gt "$SESSION_MAX" ] \
	&& mv -f "$LOGS/slot-session.log" "$LOGS/slot-session.log.1"

log() {
	echo "$(date '+%F %T') $*" >> "$LOGS/slot-session.log" 2>/dev/null
}

# A GRAVY Manager update waits beside System as System.new. Once Install was pressed on the
# page it takes over here and the
# old System is kept: when the new one never clears its .trial, the next start goes back.
if [ -f "$SD/System.new/.complete" ] && [ -f "$SD/System.new/.install" ]; then
	rm -rf "$SD/System.old"
	if mv "$SD/System" "$SD/System.old"; then
		if mv "$SD/System.new" "$SD/System"; then
			rm -f "$SYS/.complete" "$SYS/.install"
			touch "$SYS/.trial"
			log "update: $(cat "$SYS/version.txt" 2>/dev/null) swapped in"
		else
			mv "$SD/System.old" "$SD/System"
		fi
	fi
	sync
elif [ -f "$SYS/.trial" ] && [ -d "$SD/System.old" ]; then
	if mv "$SD/System" "$SD/System.bad" && mv "$SD/System.old" "$SD/System"; then
		rm -rf "$SD/System.bad"
		log "update: never reached the shelf, back to $(cat "$SYS/version.txt" 2>/dev/null)"
	fi
	sync
fi

pid="$(cat "$RUN/slot-led.pid" 2>/dev/null)"
if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
	sh "$SYS/led.sh" once
	sh "$SYS/led.sh" loop > /dev/null 2>&1 &
	echo $! > "$RUN/slot-led.pid"
fi

echo "|/bin/sh $SYS/coresave.sh %p %s %e %t" > /proc/sys/kernel/core_pattern 2>/dev/null

[ -f "$LOGS/slot.log" ] && mv -f "$LOGS/slot.log" "$LOGS/slot.log.1"
log "exec $SYS/slot"

exec /lib/ld-linux-aarch64.so.1 "$SYS/slot" > "$LOGS/slot.log" 2>&1
