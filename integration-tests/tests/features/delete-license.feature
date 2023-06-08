##
# This file is part of the IVMS Online.
#
# @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
##

Feature: Licenses management

    Scenario: Deleting license
        Given There is a license "key0" for vessel "00000000-0000-0000-0000-000000000000" of customer "00000000-0000-0000-0000-000000000001" with count 2 and expiration date "2011-01-30T14:58:00+01:00"
        When I delete license "key0" for vessel "00000000-0000-0000-0000-000000000000" of customer "00000000-0000-0000-0000-000000000001"
        Then License "key0" for vessel "00000000-0000-0000-0000-000000000000" of customer "00000000-0000-0000-0000-000000000001" does not exist

    Scenario: Deleting non-existing license
        Given There is no license "key1" for vessel "00000000-0000-0000-0000-000000000002" of customer "00000000-0000-0000-0000-000000000003"
        When I delete license "key1" for vessel "00000000-0000-0000-0000-000000000002" of customer "00000000-0000-0000-0000-000000000003"
        Then License "key1" for vessel "00000000-0000-0000-0000-000000000002" of customer "00000000-0000-0000-0000-000000000003" does not exist
