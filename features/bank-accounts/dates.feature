Feature: Transaction dates
  As a household member
  I want dates in my browser's format that never wrap
  So that the transactions table is easy to scan

Background:
  Given a NAB account "Everyday" with opening balance 100.00 at close of business "30 Oct 2026"
  And the downloaded statement for "Everyday" contains:
    | Date      | Amount | Transaction Details | Balance | Category  |
    | 05 Nov 26 | -60.00 | SUPERMARKET         | 40.00   | Groceries |
  And the transactions are imported

Scenario Outline: Dates follow the browser's language
  Given the browser language is "<language>"
  When I goto /bank-accounts/everyday
  Then the date of "SUPERMARKET" is shown as "<date>"

  Examples:
    | language | date       |
    | en-AU    | 05/11/2026 |
    | en-US    | 11/5/2026  |
    | de-DE    | 5.11.2026  |

Scenario: Dates do not wrap on a narrow screen
  Given the browser window is 320 pixels wide
  When I goto /bank-accounts/everyday
  Then the date of "SUPERMARKET" is on one line
