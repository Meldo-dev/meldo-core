# Security

Found a security problem in Meldo? Please don't open a public issue. Email **contact@meldo.dev** with what you found and how to reproduce it. You'll get an answer within a week.

The core makes no network requests. It computes grades, the timetable, statistics and task rules from data on the device, which can also arrive through sync from another device. Input that makes it panic, loop or allocate without bounds counts as a security problem.

Please test only against your own accounts and devices.
