rc -c 'x=(a b); cat <<EOF
here $x doc $#x $"x $$ $x^y $y
EOF'
rc -c 'x=(a b); cat <<'"'"'EOF'"'"'
here $x doc
EOF'
rc -c 'cat <<EOF
$1 $2 $3
EOF' one two
rc -c 'cat <<A <<B
a
A
b
B'
rc -c 'cat <<EOF | tr a-z A-Z
lower
EOF'
rc -c 'cat <<EOF >hf; cat hf
x
EOF'
rc -c 'fn f {cat <<EOF
in fn $1
EOF
}
f a
f b'
rc -c 'cat <<EOF
no terminator'
rc -c 'cat << (a b)
x
a b'
rc -c 'x=1; cat <<EOF
$x$x $x^$x $$$x
EOF'
rc -c 'cat <<EOF
héllo wörld ¢
EOF'
rc -c 'cat <<EOF
$
$ x $
EOF'
