# Week 3 Assignment - Valentin Fernandez
Polkadot Blockchain Academy B.A. - 2023


- [Project Description](#project-description)
- [Diagram](#use-case-diagram)
- [Details and Future Improvements](#details-and-future-improvements)
- [Resources](#resources)

## **Quadratic Voting Pallet**
The chosen project was the Quadratic Voting Pallet, built on Substrate for the Polkadot Blockchain Academy.
### **Project Description**
This project features the implementation of Quadratic Voting. Registered voters can submit proposals on the blockchain for voting by the rest of the voters. 

Proposals include a hash description and a time limit (in blocks) indicating when the proposal ends. 

Voters can vote "Aye" or "Nay" with a specified number of points, where the cost of each point increases quadratically. The cost is reserved from the voter's balance via the **`Reservable Currency`** trait. At any given time, multiple proposals can be ongoing. Upon reaching the time limit, a voter can finish the proposal and calculate the result. After the proposal completion, voters can unlock their reserved balances.

For this there's a list of extrinsics that allows the users tto interact with the state machine in different ways.
  + **Register Users:**
    1. `register_voter(origin, who)`
  + **Proposals:**
    1. `make_proposal(origin, description, time_period)`
    2. `increase_proposal_time(origin,	proposal_id, new_time_period)`
    3. `cancel_proposal(origin, proposal_id)`
    4. `finish_proposal(origin, proposal_id)`
  + **Voting**`
    1. `vote(origin, proposal_id, vote_decision)`
    2. `update_vote(origin, proposal_id, new_vote_decision)`
    3. `cancel_vote(origin, proposal_id)`
    4. `unlock_balance(origin, proposal_id)`

### **Use Case Diagram**
![Use case diagram](./substrate-node-template//pallets/voting/assets/Diagram.png)
### **Details and Future Improvements**
To design this pallet some decisions had to be made to define the details of the implementation. Some of this are:
  + **Reservable Currency over Lockable Currency:** The reservable currency trait was used to lock de balance of the user. Reservable currency allows for locking the balance of users to prevent double voting. This approach is more secure as it eliminates the need to individually manage each lock.
  + **Reduction Threshold:** There is a threshold during the voting period of a proposal where no reductions or cancellations of votes are allowed. This prevents voters who are aware that the proposal is likely to pass (or fail) from retrieving their vote and potentially altering the outcome.
  + **Proposals end at any time:** In some quadratic voting implementations, multiple proposals are placed in the same time window and the one with the most votes wins. However, allowing each user to choose their own end time and having proposals run independently offers greater flexibility for systems utilizing this pallet.
  + **Voters Close Proposals:** To avoid the need for an `on_initialize()`hook to close an unknown number of proposals at a certain block, the system allows voters to close proposals. This is because they have the incentive to close them to unlock their balance.

#### Some Future Considerations:
Due to a lack of time, I didn't manage to implement benchmarking to to determine the weights of each extrinsic. This is something to improve in the future.

Also a better "de-sybil" system could be implemented to avoid needing to have a centralized entity (in this case the root user) that registers users as voters. One solution would be to implement the `Identity pallet`.

And last I would like to implement a more cost-effective voting system, where multiple users' signed votes can be collected in one extrinsic and sent to the chain, reducing the cost by eliminating the need for each user to pay an individual fee. This concept is inspired by a platform in the Ethereum space called [Snapshot](https://docs.snapshot.org/).

### **Resources**
Here's some of the resources used to design and implement this pallet.
* [Democracy Pallet](https://docs.rs/pallet-democracy/13.0.0/pallet_democracy/)
* [Collective Pallet](https://paritytech.github.io/substrate/master/pallet_collective/index.html)
* [Visual demostration of Quadratic Voting](https://www.economist.com/interactive/2021/12/18/quadratic-voting)
* [A Simple Guide to Quadratic Voting](https://blog.tally.xyz/a-simple-guide-to-quadratic-voting-327b52addde1)
* [Quadratic Voting: Cross between a voting budget and a one-person-one-vote system](https://www.youtube.com/watch?v=fhVt-1cA23U)
* [Substrate Docs](https://docs.substrate.io/build/runtime-storage/)
* [Substrate Rust docs](https://paritytech.github.io/substrate/master/sc_service/index.html) 
* [Snapshot platform](https://docs.snapshot.org/)
* [Excalidraw - Diagrams Platform](https://excalidraw.com/)