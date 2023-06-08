##
# This file is part of the IVMS Online.
#
# @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
##

Feature: Licenses management

    Scenario: Listing licenses
        Given There is a license "key5" for vessel "00000000-0000-0000-0000-00000000000a" of customer "00000000-0000-0000-0000-00000000000b" with count 2 and expiration date "2011-01-30T14:58:00+01:00"
        Given There is a license "key6" for vessel "00000000-0000-0000-0000-00000000000a" of customer "00000000-0000-0000-0000-00000000000b" with count 1 and expiration date "2015-07-02T03:20:00+02:00"
        When I list licenses for vessel "00000000-0000-0000-0000-00000000000a" of customer "00000000-0000-0000-0000-00000000000b"
        Then I can read list of 2 licenses
        And License at position 0 has key "key5"
        And License at position 1 has key "key6"

    Scenario: Listing licenses next page
        Given There is a license "key7" for vessel "00000000-0000-0000-0000-00000000000c" of customer "00000000-0000-0000-0000-00000000000d" with count 4 and expiration date "2017-11-11T16:00:00+02:00"
        Given There is a license "key8" for vessel "00000000-0000-0000-0000-00000000000c" of customer "00000000-0000-0000-0000-00000000000d" with count 3 and expiration date "2009-03-23T10:00:00+02:00"
        When I list licenses for vessel "00000000-0000-0000-0000-00000000000c" of customer "00000000-0000-0000-0000-00000000000d" with page token "key7"
        Then I can read list of 1 licenses
        And License at position 0 has key "key8"
