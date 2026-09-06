#!/usr/bin/perl
# timeout.pl SECS cmd args... : run cmd in its own process group, killing
# the whole group if it runs longer than SECS.  Exits 124 on timeout.
use strict;
my $secs = shift @ARGV;
my $pid = fork();
die "fork: $!" unless defined $pid;
if ($pid == 0) {
	setpgrp(0, 0);
	exec @ARGV or die "exec: $!";
}
local $SIG{ALRM} = sub { kill 9, -$pid; waitpid($pid, 0); exit 124 };
alarm $secs;
waitpid($pid, 0);
my $st = $?;
alarm 0;
exit(($st & 127) ? 128 + ($st & 127) : ($st >> 8));
