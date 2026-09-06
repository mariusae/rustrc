rc -c 'eval echo one; eval "x=1; echo" '"'"'$x'"'"'; eval; echo st=$status'
rc -c 'eval echo `{echo a b}; x=(echo a; echo b); eval $x'
rc -c 'eval ")"; echo after; echo st=$status'
rc -c 'eval "echo a" "echo b"; eval "echo a;" "echo b"'
rc -c 'exec echo replaced; echo notreached'; echo "code=$?"
rc -c 'exec nonexistent; echo after'; echo "code=$?"
rc -c 'exec; echo after'; echo "code=$?"
rc -c 'exec >f1; echo to-file; exec >[1=2]; echo to-err' 2>/dev/null; cat f1 2>/dev/null || true
rc -c 'exit 2; echo x'; echo "code=$?"
rc -c 'wait; echo st=$status; sleep 0.1 & wait $apid; echo st=$status; {sleep 0.1; exit 3} & wait $apid; echo st=$status; wait 99999; echo st=$status; wait a b; echo st=$status'
rc -c '{sleep 0.1; exit 3} & {sleep 0.2; exit 4} & wait; echo st=$status'
rc -c 'x=a; whatis x; x=(a b); whatis x x'
rc -c 'umask 022; umask; umask 077; umask; umask 8; umask a b; umask -x'
rc -c 'ulimit -n 2>&1 | sed "s/[0-9][0-9]*/N/"; ulimit -a | sed "s/ [0-9][0-9]*$/ N/"; ulimit -z; ulimit -n a; ulimit -n 1 2; echo st=$status'
rc -c 'ulimit -H -n | sed "s/[0-9][0-9]*/N/"; ulimit -S -c 0; ulimit -c; ulimit -SH -c 0; ulimit -c unlimited; ulimit -c; echo st=$status'
rc -c 'rfork; echo st=$status; rfork e; echo st=$status; rfork s; echo st=$status; rfork n; echo st=$status; rfork f; echo st=$status; rfork x; echo st=$status; rfork a b; echo st=$status'
rc -c 'finit; echo st=$status'
rc -c 'x=(a b); . -i /dev/null a; echo $#*'
rc -c 'nonexistent; echo st=$status; /nonexistent; echo st=$status; ./nonexistent; echo st=$status'; echo "code=$?"
rc -c 'script.rc; echo st=$status; ./script.rc; echo st=$status'
rc -c 'chmod +x script.rc; ./script.rc; echo st=$status'
rc -c 'shebang arg; echo st=$status'
rc -c 'echo `{shebang q}'
rc -c 'builtin echo hi; builtin exit 3'; echo "code=$?"
rc -c 'fn exit {echo trapped}; exit 1; builtin exit 2'; echo "code=$?"
rc -c 'wait ; echo ok'
rc -c 'cd; whatis cd; . /dev/null'
rc -c 'x=1 echo $x >/dev/null; whatis x'
rc -c '. -i; echo st=$status'
rc -c 'exit "'; echo "code=$?"
