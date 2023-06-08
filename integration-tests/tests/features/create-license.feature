##
# This file is part of the IVMS Online.
#
# @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
##

Feature: Licenses management

    Scenario: Creating licenses
        When I create license "key11" for vessel "00000000-0000-0000-0000-000000000001" of customer "00000000-0000-0000-0000-000000000005" with count 7 and expiration date "2011-01-30T14:58:00+01:00"
        Then I can read license key
        And License with that key exists with count 7 and expiration date "2011-01-30T14:58:00+01:00"
