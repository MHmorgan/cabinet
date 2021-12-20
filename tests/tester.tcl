package require tcltest

namespace import tcltest::makeFile tcltest::removeFile \
                 tcltest::makeDirectory tcltest::removeDirectory \
                 tcltest::cleanupTests

proc cargo {args} {
  exec -ignorestderr -- cargo {*}$args 2>@1
}

################################################################################
#                                                                              #
# Cabinet procedures
#                                                                              #
################################################################################

set cabinet_pid  0
set cabinet_host 0.0.0.0
set cabinet_port 8083
set cabinet_log [file normalize cabinet.log]

proc start_cabinet {} {
  global cabinet_pid cabinet_host cabinet_port cabinet_log
  set cabinet_pid [exec [cabinet_bin] $cabinet_host $cabinet_port >>& $cabinet_log &]
  log "Started cabinet server (PID $cabinet_pid)"
}

proc teardown_cabinet {} {
  global cabinet_pid
  log "Tearing down cabinet server"
  if {!$cabinet_pid} {
    throw {TESTER} "Cabinet PID unknown"
  }
  exec kill -9 $cabinet_pid
  file delete cabinet.sqlite
  cleanupTests
}

proc cabinet_bin {} {
  set path [file normalize ../target/release/cabinet]
  if {![file exists $path]} {
    throw {TESTER} "Cabinet binary not found: $path"
  } elseif {![file executable $path]} {
    throw {TESTER} "Cabinet binary not executable: $path"
  }
  return $path
}

proc cabinet {args} {
  exec -ignorestderr -- [cabinet_bin] {*}$args
}

proc cabinet_url {} {
  global cabinet_host cabinet_port
  return http://$cabinet_host:$cabinet_port
}


################################################################################
#                                                                              #
# HTTP procedures
#                                                                              #
################################################################################

# get PATH ?HEADERS?
#
#   Perform a GET request to the cabinet server.
#
# Arguments:
#   PATH    Path to the resource.
#   HEADERS Request headers. A key-value list.
#
proc get {path {headers {}}} {
  http::geturl [cabinet_url]/$path \
    -method GET \
    -headers $headers
}

# head PATH ?HEADERS?
#
#   Perform a HEAD request to the cabinet server.
#
# Arguments:
#   PATH    Path to the resource.
#   HEADERS Request headers. A key-value list.
#
proc head {path {headers {}}} {
  http::geturl [cabinet_url]/$path \
    -method HEAD \
    -headers $headers
}

# put PATH ?BODY? ?HEADERS?
#
#   Perform a PUT request to the cabinet server.
#
# Arguments:
#   PATH    Path to the resource.
#   BODY    Request body.
#   HEADERS Request headers. A key-value list.
#
proc put {path {body {}} {headers {}}} {
  http::geturl [cabinet_url]/$path \
    -method PUT \
    -headers $headers \
    -query $body
}

# delete PATH ?HEADERS?
#
#   Perform a HEAD request to the cabinet server.
#
# Arguments:
#   PATH    Path to the resource.
#   HEADERS Request headers. A key-value list.
#
proc delete {path {headers {}}} {
  http::geturl [cabinet_url]/$path \
    -method DELETE \
    -headers $headers
}

# http_time TIMEVAL
#
#   Format a time value as required by HTTP: Wed, 21 Oct 2015 07:28:00 GMT
#
# Arguments:
#   TIMEVAL  Integer number of seconds
#
proc http_time {timeval} {
  clock format $timeval -format {%a, %d %b %Y %T GMT}
}

################################################################################
#                                                                              #
# Generic utilites
#                                                                              #
################################################################################

proc diff {txt1 txt2} {
  set tmpdir [tcltest::configure -tmpdir]
  makeFile $txt1 file1 $tmpdir
  makeFile $txt2 file2 $tmpdir
  try {
    set txt [exec diff $tmpdir/file1 $tmpdir/file2]
  } trap {CHILDSTATUS} {result options} {
    set txt $result
  }
  removeFile file1 $tmpdir
  removeFile file2 $tmpdir
  return $txt
}

proc touch {name} {
  exec -- touch $name
}
