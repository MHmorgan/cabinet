#!/usr/bin/env tclsh

set TEST_DIR [file dirname $argv0]
cd $TEST_DIR

package require tcltest
package require Thread

source common.tcl
source tester.tcl

namespace import common::*

# Build binaries
log "Bulding cabinet..."
try {
  set cmd {cargo build --release --bin cabinet}
  puts $cmd
  {*}$cmd
} on error {msg} {
  bail "Failed to build:\n$msg"
}

# set pid [start_cabinet]

# Setup constraints
set constraints {always files dirs boilerplates}
log "Enabled test constraints: $constraints"

tcltest::configure \
  -constraints $constraints \
  -verbose {body error msec skip}
tcltest::runAllTests
