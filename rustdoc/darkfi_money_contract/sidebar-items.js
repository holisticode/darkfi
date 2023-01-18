window.SIDEBAR_ITEMS = {"constant":[["MONEY_CONTRACT_COIN_MERKLE_TREE",""],["MONEY_CONTRACT_COIN_ROOTS_TREE",""],["MONEY_CONTRACT_FAUCET_PUBKEYS",""],["MONEY_CONTRACT_FIXED_SUPPLY_TREE",""],["MONEY_CONTRACT_INFO_TREE",""],["MONEY_CONTRACT_LEAD_COIN_MERKLE_TREE",""],["MONEY_CONTRACT_LEAD_COIN_ROOTS_TREE",""],["MONEY_CONTRACT_LEAD_INFO_TREE",""],["MONEY_CONTRACT_LEAD_NULLIFIERS_TREE",""],["MONEY_CONTRACT_NULLIFIERS_TREE",""],["MONEY_CONTRACT_ZKAS_BURN_NS_V1","zkas burn contract namespace"],["MONEY_CONTRACT_ZKAS_LEAD_BURN_NS_V1","zkas staking coin burn contract namespace"],["MONEY_CONTRACT_ZKAS_LEAD_MINT_NS_V1","zkas staking coin mint contract namespace"],["MONEY_CONTRACT_ZKAS_MINT_NS_V1","zkas mint contract namespace"],["MONEY_CONTRACT_ZKAS_TOKEN_MINT_NS_V1","zkas token mint contract namespace"]],"enum":[["MoneyFunction","Functions we allow in this contract"]],"mod":[["client","Transaction building API for clients interacting with this contract. This module implements the client-side of this contract’s interaction. What we basically do here is implement an API that creates the necessary structures and is able to export them to create a DarkFi Transaction object that can be broadcasted to the network when we want to make a payment with some coins in our wallet. Note that this API doesn’t involve any wallet interaction, but only takes the necessary objects provided by the caller. This is so we can abstract away the wallet interface to client implementations."],["model","Structures and object definitions"]]};