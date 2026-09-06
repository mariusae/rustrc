rc -c 'true; kill -TERM $pid; echo after'; echo "code=$?"
rc -c 'false; kill -TERM $pid; echo after'; echo "code=$?"
rc -c 'fn sigkill {echo caught}; kill -TERM $pid; echo after'; echo "code=$?"
rc -c 'fn sigkill {echo caught $*}; kill -TERM $pid; echo after' a b; echo "code=$?"
rc -c 'fn sigint {echo int}; kill -INT $pid; echo after'; echo "code=$?"
rc -c 'kill -INT $pid; echo after'; echo "code=$?"
rc -c 'fn sighup {echo hup}; kill -HUP $pid; echo after'; echo "code=$?"
rc -c 'kill -HUP $pid; echo after'; echo "code=$?"
rc -c 'fn sigalrm {echo alrm}; kill -ALRM $pid; echo after'; echo "code=$?"
rc -c 'kill -USR1 $pid; echo after' 2>&1; echo "code=$?"
rc -c 'kill -PIPE $pid; echo after' 2>&1; echo "code=$?"
rc -c 'kill -WINCH $pid; echo after' 2>&1; echo "code=$?"
rc -c 'kill -TSTP $pid; echo after' 2>&1; echo "code=$?"
rc -c 'fn sigint {echo int $#n; n=($n x); if(! ~ $#n 3) kill -INT $pid}; kill -INT $pid; echo after' 2>&1; echo "code=$?"
rc -c 'fn sigexit {echo exit}; kill -TERM $pid; echo after' 2>&1; echo "code=$?"
rc -c 'fn sigexit {echo exit}; fn sigkill {echo k}; kill -TERM $pid; echo after' 2>&1; echo "code=$?"
rc -c 'fn sigint {echo int}; @{kill -INT $pid}; echo after' 2>&1; echo "code=$?"
rc -c 'fn sigint {echo int}; {kill -INT $pid; echo inbrace} | cat; echo after' 2>&1; echo "code=$?"
rc -c 'sh -c "kill -TERM \$\$"; echo st=$status' 2>&1; echo "code=$?"
rc -c 'sh -c "kill -SEGV \$\$"; echo st=$status' 2>&1; echo "code=$?"
rc -c 'sh -c "kill -TERM \$\$" | cat; echo st=$status' 2>&1; echo "code=$?"
rc -c 'sh -c "kill -TERM \$\$" & wait; echo st=$status' 2>&1; echo "code=$?"
printf 'sh -c "kill -TERM \\$\\$"\necho st=$status\n' | rc -i 2>&1; echo "code=$?"
printf 'sh -c "kill -TERM \\$\\$" &\nwait\necho st=$status\n' | rc -i 2>&1; echo "code=$?"
printf 'kill -INT $pid\necho after\n' | rc -i 2>&1; echo "code=$?"
printf 'kill -INT $pid\necho after\n' | rc -I 2>&1; echo "code=$?"
printf 'fn sigint {echo int}\nkill -INT $pid\necho after\n' | rc -i 2>&1; echo "code=$?"
rc -c 'fn sigexit {echo bye}; sh -c "kill -TERM \$\$"; echo st=$status' 2>&1; echo "code=$?"
rc -c 'x=1 & echo apid=$apid; wait; echo st=$status'
rc -c 'sleep 0.1 & echo apid=$apid; echo $#apid'
rc -c '{exit 5} & wait $apid; echo st=$status'
rc -c 'echo hi & wait'
rc -c 'echo hi | cat &
wait'
rc -c 'true & false & wait; echo st=$status'
rc -c 'fn sigint {echo caught; exit 7}; kill -INT $pid; echo after'; echo "code=$?"
