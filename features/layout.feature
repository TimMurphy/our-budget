Feature: Website layout
  As a household member
  I want a modern website
  So I can easily maintain a budget

Scenario: Home page
  Given a NAB account "Everyday" with opening balance 100.00 at close of business "30 Oct 2026"
  And the transactions are imported
  When I goto /
  Then the top row is fixed to the top of the browser
  And the top left of the screen displays "Our Budget"
  And the right side contains a navbar with "Bank Accounts" and "Budgets"
  And the page background colour is "#191919"
  And the navbar and cards background colour is "#303030"
  And the foreground colour is "#bae2ef"
  And the body contains a card with heading "Bank Accounts"
  And the "Bank Accounts" card lists "Everyday"
