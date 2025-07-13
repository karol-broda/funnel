#!/bin/sh
# this script is a wrapper around the main application binary.
# it allows us to use environment variables to configure the application's command-line flags.

set -e

# if the first argument is a flag, assume the user wants to run the server.
# in this case, we prepend the server command to the arguments.
if [ "${1#-}" != "$1" ]; then
  set -- /usr/local/bin/funnel-server "$@"
fi

# execute the command.
# this will be "funnel-server --host ..." if the user runs the container without arguments,
# or a command like "sh" if the user wants an interactive shell.
exec "$@" 