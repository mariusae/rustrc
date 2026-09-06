python3 -c "print('echo ' + 'a'*8190 + '; echo after')" | rc | wc -c
python3 -c "print('echo ' + 'a'*8191 + '; echo after')" | rc 2>&1 | wc -c
python3 -c "print('echo ' + 'a'*8191 + '; echo after')" | rc 2>&1 | head -c 100; echo
python3 -c "print('echo ' + 'a'*8191 + '; echo after')" | rc 2>&1 | tail -c 60; echo
python3 -c "print('echo ' + 'a'*20000 + '; echo after')" | rc 2>&1 | tail -c 60; echo
python3 -c "print('echo ' + \"'\" + 'a'*8191 + \"'\" + '; echo after')" | rc 2>&1 | tail -c 60; echo
python3 -c "print('x=' + 'a'*100 + '\necho \$x | wc -c')" | rc
python3 -c "import sys; sys.stdout.write('cat <<EOF\n' + 'a'*5000 + '\nEOF\n')" | rc | wc -c
python3 -c "import sys; sys.stdout.write('cat <<EOF\n' + 'a'*4095 + '\nEOF\n')" | rc | wc -c
python3 -c "import sys; sys.stdout.write('x=1; cat <<EOF\n' + 'a'*4095 + '\$x\nEOF\n')" | rc | tail -c 5; echo
python3 -c "import sys; sys.stdout.write('x=1; cat <<EOF\n' + 'a'*4094 + '\$x\nEOF\n')" | rc | tail -c 5; echo
python3 -c "print('echo `{printf \"%s\" ' + 'a'*9000 + '} | wc -c')" | rc
python3 -c "print('x=`{printf \"%s\" ' + 'a'*9000 + '}; echo \$#x')" | rc
