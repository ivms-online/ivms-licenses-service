##
# This file is part of the IVMS Online.
#
# @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
##

Feature: Licenses management

    Scenario: Fetching license
        Given There is a license "key9" for vessel "00000000-0000-0000-0000-000000000006" of customer "00000000-0000-0000-0000-000000000007" with count 2 and expiration date "2011-01-30T14:58:00+01:00"
        When I fetch license "key9" for vessel "00000000-0000-0000-0000-000000000006" of customer "00000000-0000-0000-0000-000000000007"
        Then I can read license key as "key9"
        Then I can read license count as 2
        Then I can read license expiration date as "2011-01-30T14:58:00+01:00"

    Scenario: Fetching non-existing license
        Given There is no license "key10" for vessel "00000000-0000-0000-0000-000000000008" of customer "00000000-0000-0000-0000-000000000009"
        When I fetch license "key10" for vessel "00000000-0000-0000-0000-000000000008" of customer "00000000-0000-0000-0000-000000000009"
        Then I get "License not found." API error response
