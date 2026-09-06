rc -c 'echo -$status-; true; echo -$status-; false; echo -$status-'
rc -c 'false | true; echo $status; true | false; echo $status; true | true | false; echo $status'
rc -c '~ a a; echo $status; ~ a b; echo $status; ! ~ a b; echo $status; ! true; echo $status; ! false; echo $status'
rc -c 'x=1; ~ 1 $x; echo $status'
rc -c 'exit 3'; echo "code=$?"
rc -c 'exit foo'; echo "code=$?"
rc -c 'exit 0'; echo "code=$?"
rc -c 'false; exit'; echo "code=$?"
rc -c 'exit'; echo "code=$?"
rc -c 'false | true; exit'; echo "code=$?"
rc -c 'true | false; exit'; echo "code=$?"
rc -c 'exit 256'; echo "code=$?"
rc -c 'exit -1'; echo "code=$?"
rc -c 'exit 1 2'; echo "code=$?"
rc -c 'status=foo; echo $status; exit'; echo "code=$?"
rc -c 'status=(a b); echo $status; exit'; echo "code=$?"
rc -c 'echo `{false}; echo $status'
rc -c 'echo `{exit 7}; echo $status'
rc -c 'if(true) echo t; echo $status'
