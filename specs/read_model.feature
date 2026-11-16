Feature: Read model tables
  The read models are tables in the database, updated in the same transaction as the events they
  reflect. Starting the application applies only the events stored since, instead of replaying every event.

  Scenario: Restarting loads the read model tables without replaying events
    Given today is "2026-11-16"
    And a Westpac bank account "Joint" with opening balance "$1,000.00"
    And the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -50.00 | Coffee      |
      | 2026-11-03 | -20.00 | Bus         |
    And the statements have been imported
    And I assign "Coffee" in "Joint" to "Eating Out"
    And I budget "$100.00" for "Eating Out" in "2026-11"
    Then the transactions table has 2 rows
    And the assignments table has 1 row
    And the bank_accounts table has 1 row
    When the application is restarted
    Then the read model was loaded without replaying events
    And "Joint" has balance "$930.00"
    And "Joint" has 2 transactions
    And "Coffee" in "Joint" is assigned "-$50.00" to "Eating Out"
    And in "2026-11" budget account "Eating Out" shows budgeted "$100.00", spent "-$50.00" and available "$50.00"

  Scenario: Only events stored after the saved read model are applied
    Given a Westpac bank account "Joint" with opening balance "$0.00"
    And an event is stored without updating the read model
    When the application is restarted
    Then 1 newer event was applied to the read model tables
    And budget account "Extra" is in group "Ungrouped"
    When the application is restarted
    Then the read model was loaded without replaying events

  Scenario: A read model with no saved copy is rebuilt from the events once
    Given a Westpac bank account "Joint" with opening balance "$500.00"
    And the read model state is removed
    When the application is restarted
    Then the read model was rebuilt from the events
    And "Joint" has balance "$500.00"
    When the application is restarted
    Then the read model was loaded without replaying events

  Scenario: A read model that no longer matches the stored events is rebuilt
    Given a Westpac bank account "Joint" with opening balance "$500.00"
    And a stored event is deleted behind the application's back
    When the application is restarted
    Then the read model was rebuilt from the events

  Scenario: Account events stored with an opening balance in them still give that balance
    Given an account event in the old format with opening balance "$123.45" is stored
    When the application is restarted
    Then "Old account" has balance "$123.45"

  Scenario: Events replaced behind the application's back are noticed even when the count is unchanged
    Given a Westpac bank account "Joint" with opening balance "$500.00"
    And a Westpac bank account "Other" with opening balance "$0.00"
    And the first stored event is swapped for a copy of the next one
    When the application is restarted
    Then the read model was rebuilt from the events
