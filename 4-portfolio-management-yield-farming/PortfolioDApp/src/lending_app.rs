use scrypto::prelude::*;
use crate::utils::*;


// Here, we define the data that will be present in
// each of the lending ticket NFTs.
#[derive(NonFungibleData)]
struct LendingTicket {
    #[scrypto(mutable)]
    number_of_lendings: i32,
    #[scrypto(mutable)]   
    l1: bool,
    #[scrypto(mutable)]   
    l2: bool,
    #[scrypto(mutable)]   
    in_progress: bool          
}

// Here, we define the data that will be present in
// each of the borrowing ticket NFTs. 
#[derive(NonFungibleData)]
struct BorrowingTicket {
    #[scrypto(mutable)]
    number_of_borrowings: i32,
    #[scrypto(mutable)]   
    xrds_to_give_back: Decimal,    
    #[scrypto(mutable)]   
    l1: bool,
    #[scrypto(mutable)]   
    l2: bool,
    #[scrypto(mutable)]   
    in_progress: bool          
}

blueprint! {
    #[derive(Debug)]
    struct LendingApp {
        /// The resource definition of LOAN token.
        loan_resource_def: ResourceAddress,
        /// The resource definition of LENDING_NFT token.
        lending_nft_resource_def: ResourceAddress,
        /// The resource definition of BORROWING_NFT token.
        borrowing_nft_resource_def: ResourceAddress,                
        /// LOAN tokens mint badge.
        loan_admin_badge: Vault,       
        /// LOAN tokens Vault.
        loan_pool: Vault,          

        /// The reserve for main pool
        main_pool: Vault,

        ///loans along time
        loan_allocated: Vec<Decimal>,
        /// The starting amount of token accepted
        start_amount: Decimal,
        /// The fee to apply for every loan
        fee: Decimal,
        /// The bonus_fee for level l1
        bonus_fee_l1: Decimal,
        /// The bonus_fee for level l2
        bonus_fee_l2: Decimal,                
        /// The reward to apply for every loan
        reward: Decimal,
        /// The extra_reward for level l1
        extra_reward_l1: Decimal,
        /// The extra_reward for level l2
        extra_reward_l2: Decimal,                
        /// Min ratio a lender can lend
        min_ratio_for_lend: Decimal,
        /// Max ratio a lender can lend
        max_ratio_for_lend: Decimal,
        /// Min ratio a borrower can borrow
        min_ratio_for_borrow: Decimal,
        /// Max ratio a borrower can borrow
        max_ratio_for_borrow: Decimal,        
        /// Loan pool low limit ratio
        loan_pool_low_limit: Decimal,        
        /// Main pool low limit ratio
        main_pool_low_limit: Decimal,        
        /// The cumulative amount borrowed
        cumulative: Decimal
    }



    impl LendingApp {
        /// Creates a LendingApp component and returns the component address
        pub fn instantiate_pool(
            starting_tokens: Bucket,
            start_amount: Decimal,
            fee: Decimal,
            reward: Decimal,
        ) -> ComponentAddress {
            info!("Starting amount is: {}", start_amount);
            info!("Fee for borrower is: {}", fee);
            info!("Reward for lenders is: {}", reward);
            // Check arguments 
            assert!(
                (fee >= dec!("5")) & (fee <= dec!("10")), 
                "Invalid fee : Fee must be between 5 and 10"
            );
            assert!(
                (reward >= dec!("3")) & (reward <= dec!("7")), 
                "Invalid reward : Reward must be between 3 and 7"
            );       
            assert!(
                (reward < fee), 
                "Invalid fee / reward : Fee must be higher than reward"
            );                      
            assert!(
                start_amount >= dec!("1000"),
                "Loan Pool must start with at least 1000 XRD tokens !"
            );                  
            //assert!(
            //    starting_tokens.resource_address().to_string() == scrypto::constants::RADIX_TOKEN,
            //    "[Main Pool Creation]: Main Pool may only be created with XRD tokens."
            //);
            assert!(
                !starting_tokens.is_empty() , 
                "[Main Pool Creation]: Can't create a pool from an empty bucket."
            );

            // Create the loan admin badge. This will be store on the component's vault 
            // and will allow it to do some actions on the user NFTs
            let loan_admin_badge: Bucket = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", "Loan Token Auth")
                .metadata("description", "Authorizes the withdraw of the nft and the updates of metadata")
                .initial_supply(1);

            // let loan_admin_id_badge = ResourceBuilder::new_non_fungible()
            //     .metadata("name", " Lending ID badge")
            //     .mintable(rule!(require(loan_admin_badge.resource_address())), LOCKED)
            //     .burnable(rule!(require(loan_admin_badge.resource_address())), LOCKED)
            //     .restrict_withdraw(rule!(deny_all), LOCKED)
            //     .no_initial_supply();                                
                
            // Create the non fungible resource that will represent the lendings
            let lending_nft: ResourceAddress = ResourceBuilder::new_non_fungible()
                .metadata("name", "Lending NFTs")
                .mintable(rule!(require(loan_admin_badge.resource_address())), LOCKED)
                .burnable(rule!(require(loan_admin_badge.resource_address())), LOCKED)
                .updateable_non_fungible_data(rule!(require(loan_admin_badge.resource_address())), LOCKED)
                .restrict_withdraw(rule!(deny_all), MUTABLE(rule!(require(loan_admin_badge.resource_address()))))
                // .restrict_withdraw(rule!(require(loan_admin_id_badge)), MUTABLE(rule!(require(loan_admin_badge.resource_address()))))
                .no_initial_supply();                

            // Create the non fungible resource that will represent the borrowings
            let borrowing_nft: ResourceAddress = ResourceBuilder::new_non_fungible()
                .metadata("name", "Borrowing NFTs")
                .mintable(rule!(require(loan_admin_badge.resource_address())), LOCKED)
                .burnable(rule!(require(loan_admin_badge.resource_address())), LOCKED)
                .updateable_non_fungible_data(rule!(require(loan_admin_badge.resource_address())), LOCKED)
                .restrict_withdraw(rule!(deny_all), MUTABLE(rule!(require(loan_admin_badge.resource_address()))))
                // .restrict_withdraw(rule!(require(loan_admin_id_badge)), MUTABLE(rule!(require(loan_admin_badge.resource_address()))))
                .no_initial_supply();                 

            let loan_tokens = ResourceBuilder::new_fungible()
                .metadata("symbol", "LND")
                .metadata("name", "Loan token")
                .metadata("url", "https://lendingapp.com")
                .mintable(rule!(require(loan_admin_badge.resource_address())), LOCKED)
                .burnable(rule!(require(loan_admin_badge.resource_address())), LOCKED)
                .initial_supply(start_amount);

            let loan_allocated = Vec::new();

            info!("Loan pool size is: {}", start_amount);
            info!("Main pool size is: {}", starting_tokens.amount());

            // Instantiate our LendingApp component
            let lendingapp = Self {
                loan_resource_def: loan_tokens.resource_address(),
                lending_nft_resource_def: lending_nft,
                borrowing_nft_resource_def: borrowing_nft,
                loan_admin_badge: Vault::with_bucket(loan_admin_badge),
                loan_pool: Vault::with_bucket(loan_tokens),
                main_pool: Vault::with_bucket(starting_tokens),
                loan_allocated,
                start_amount,
                fee,
                bonus_fee_l1: dec!("0.4"),
                bonus_fee_l2: dec!("0.8"),
                reward,
                extra_reward_l1: dec!("0.4"),
                extra_reward_l2: dec!("0.8"),
                min_ratio_for_lend: dec!("5"),
                max_ratio_for_lend: dec!("20"),
                min_ratio_for_borrow: dec!("3"),
                max_ratio_for_borrow: dec!("12"),
                loan_pool_low_limit: dec!(75),
                main_pool_low_limit: dec!(50),
                cumulative: Decimal::zero(),
            }
            .instantiate();
            //order of resources build is that of the order created
            //Admin Badge, Lend NFT, Borrow NFT, LND token

            // Return the new LendingApp component, as well as the initial supply of LP tokens
            lendingapp.globalize()
        }


        // Allow someone to register its account for lendings
        // Account receives back a Soulbound token (Lending NFT)
        pub fn register(&self) -> Bucket {
            let lend_id = get_non_fungible_id();              

            // Create a lending NFT. Note that this contains the number of lending and the level arwarded
            let lending_nft = self.loan_admin_badge.authorize(|| {
                borrow_resource_manager!(self.lending_nft_resource_def)
                    .mint_non_fungible(&lend_id, LendingTicket {number_of_lendings: 0, l1: false, l2: false, in_progress: false  })
                }); 

            info!("Min/Max Ratio for lenders is: {}  {}", self.min_ratio_for_lend,  self.max_ratio_for_lend);
            info!("Extra L1 reward is : {} and L2 reward is : {}", self.extra_reward_l1, self.extra_reward_l2);
            info!("Lending NFT resource address : {} ", lending_nft.resource_address());

            // Return the NFT
            lending_nft
        }

        // Allow someone to register its account for borrowings
        // Account receives back a Soulbound token (Borrowing NFT)
        pub fn register_borrower(&self) -> Bucket {
            let borrow_id = get_non_fungible_id();                

            // Create a borrowing NFT. Note that this contains the number of borrowing and the level arwarded
            let borrowing_nft = self.loan_admin_badge.authorize(|| {
                borrow_resource_manager!(self.borrowing_nft_resource_def)
                    .mint_non_fungible(&borrow_id, BorrowingTicket {number_of_borrowings: 0, xrds_to_give_back: Decimal::zero(), l1: false, l2: false, in_progress: false  })
                }); 

            info!("Min/Max Ratio for borrowers is: {}  {}", self.min_ratio_for_borrow,  self.max_ratio_for_borrow);
            info!("Bonus L1 fee is : {} and L2 is : {}", self.bonus_fee_l1, self.bonus_fee_l2);                

            // Return the NFT
            borrowing_nft
        }        
        

        /// Lend XRD token to then pool and get back Loan tokens plus reward
        pub fn lend_money(&mut self, xrd_tokens: Bucket, ticket: Proof) -> Bucket {
            info!("=== LEND OPERATION START === ");
            info!("Loan pool size is: {}", self.loan_pool.amount());
            info!("Main pool size is: {}", self.main_pool.amount());
            // The ratio of added liquidity.
            let ratio = xrd_tokens.amount() * dec!("100") / self.main_pool.amount();
            info!("Low loan pool limit is: {}, Ratio of added liquidity is: {} " , pool_low_limit(self.start_amount, self.loan_pool_low_limit), ratio.floor());
            
            //check if lend is acceptable
            //bucket size has to be between 5% and 20% of the main vault size
            let min_level: Decimal = calculate_level(self.min_ratio_for_lend, self.main_pool.amount()); 
            let max_level: Decimal = calculate_level(self.max_ratio_for_lend, self.main_pool.amount()); 
            assert!(
                ratio > calculate_ratio(self.min_ratio_for_lend, self.main_pool.amount()),
                "Lend is below the minimum level, actual minimum is: {} Min tokens you can lend is {}", ratio, min_level
            );  
            assert!(
                ratio < calculate_ratio(self.max_ratio_for_lend, self.main_pool.amount()),
                "Lend is above the minimum level, actual maximum is: {} Max tokens you can lend is {}", ratio, max_level
            );               

            //check if pool vault size is above 75% 
            assert!(
                self.loan_pool.amount() > pool_low_limit(self.start_amount, self.loan_pool_low_limit),
                "Loan Pool size is below its limit, no more lendings are accepted now"
            );             
            
            //put xrd token in main pool
            let num_xrds = xrd_tokens.amount();
            self.main_pool.put(xrd_tokens);
            //calculate reward
            let reward = num_xrds + (num_xrds*self.reward/100);
            //give back lnd token plus reward %
            let mut value_backed = self.loan_pool.take((reward).round(2,RoundingMode::TowardsPositiveInfinity));
            info!("Loan token received: {} ", reward);       

            // Get the data associated with the Lending NFT and update the variable values
            let non_fungible: NonFungible<LendingTicket> = ticket.non_fungible();
            let mut lending_nft_data = non_fungible.data();
            //check if no operation is already in place            
            assert!(!lending_nft_data.in_progress, "You already have a lend open!");
            info!("Number of lendings is: {} - L1 : {} - L2 : {} ", lending_nft_data.number_of_lendings, lending_nft_data.l1, lending_nft_data.l2);
            //L1 if number_of_lendings between 10 and 20
            if l1_enabled(lending_nft_data.number_of_lendings,10,20) {                
                lending_nft_data.l1 = true;
                //calculate extra reward for l1 level
                let extra_reward = num_xrds*(self.extra_reward_l1)/100;
                println!("L1 reached ! extra reward assigned {}" , extra_reward);
                //give back lnd token plus reward %
                value_backed.put(self.loan_pool.take(extra_reward));
                info!("Extra Loan token received because of L1: {} ", extra_reward);
            //L2 if number_of_lendings > 20
            } else if l2_enabled(lending_nft_data.number_of_lendings,10,20) {
                lending_nft_data.l2 = true;
                //calculate extra reward for l2 level
                let extra_reward = num_xrds*(self.extra_reward_l2)/100;
                println!("L2 reached ! extra reward assigned {}" , extra_reward);
                //give back lnd token plus reward %
                value_backed.put(self.loan_pool.take(extra_reward));
                info!("Extra Loan token received because of L2: {} ", extra_reward);
            } 

            // Update the data on that NFT globally
            lending_nft_data.in_progress = true;
            self.loan_admin_badge.authorize(|| {
                borrow_resource_manager!(self.lending_nft_resource_def).update_non_fungible_data(&non_fungible.id(), lending_nft_data);
            });

            info!("New Loan pool size is: {}", self.loan_pool.amount());
            info!("New Main pool size is: {}", self.main_pool.amount());

            // Return the tokens along with NFT
            value_backed
        }

        /// Gives money back to the lenders adding their reward
        pub fn take_money_back(&mut self, lnd_tokens: Bucket, ticket: Proof) -> Bucket {
            info!("=== TAKE OPERATION START === ");
            info!("Loan pool size is: {}", self.loan_pool.amount());
            info!("Main pool size is: {}", self.main_pool.amount());            
            info!("Loan pool low limit is: {}" , pool_low_limit(self.start_amount, self.loan_pool_low_limit));
            
            // Get the data associated with the Lending NFT and update the variable values (in_progress=false)
            let non_fungible: NonFungible<LendingTicket> = ticket.non_fungible();
            let mut lending_nft_data = non_fungible.data();
            //check if no operation is already in place            
            assert!(lending_nft_data.in_progress, "You have not a lend open!");
            assert!(
                self.main_pool.amount() > pool_low_limit(self.start_amount, self.main_pool_low_limit),
                "Main pool is below limit, withdrawals must wait for Borrower repayments "
            );  
            //calculate reward
            let mut take_reward: Decimal = self.reward;
            if l1_enabled(lending_nft_data.number_of_lendings,10,20) {                
                take_reward += self.extra_reward_l1;
            //L2 if number_of_lendings > 20
            } else if l2_enabled(lending_nft_data.number_of_lendings,10,20) {
                take_reward += self.extra_reward_l2;
            } 
            //increase num of lendings            
            let number_of_lendings: i32 = 1 + lending_nft_data.number_of_lendings;
            lending_nft_data.number_of_lendings = number_of_lendings;

            // The amount of $xrd token to be repaid back (reward included)
            let how_many_to_give_back = lnd_tokens.amount();
            info!("Getting from main pool xrd tokens size: {}", how_many_to_give_back);
            //take $xrd from main pool
            let xrds_to_give_back = self.main_pool.take(how_many_to_give_back);

            //calculate what part of the received bucket cames from the original amount that was lend
            // lnd_size_received : xrd_size_returned = (100+reward) : 100
            // xrd_size_returned = (lnd_size_received * 100) / (100+reward)
            let xrd_size_returned = (how_many_to_give_back*dec!("100")/(dec!("100")+take_reward)).round(2,RoundingMode::TowardsPositiveInfinity);
            let lnd_to_be_burned = how_many_to_give_back - xrd_size_returned;
            //lnd token to put back in the pool
            info!("Putting back into loan pool lnd tokens size: {} then burning the reward because not needed anymore {} ", xrd_size_returned, lnd_to_be_burned);
            self.loan_pool.put(lnd_tokens);
            //burn the reward
            self.loan_admin_badge.authorize(|| {
                self.loan_pool.take(lnd_to_be_burned).burn();
            }); 

            info!("New Loan pool size is: {}", self.loan_pool.amount());
            info!("New Main pool size is: {}", self.main_pool.amount());
   
            lending_nft_data.in_progress = false;
            // Update the data on that NFT globally         
            self.loan_admin_badge.authorize(|| {
                borrow_resource_manager!(self.lending_nft_resource_def).update_non_fungible_data(&non_fungible.id(), lending_nft_data)
            });            

            xrds_to_give_back
        }        

        /// Borrow money to anyone requesting it, without asking for collaterals
        pub fn borrow_money(&mut self, xrd_requested: Decimal, ticket: Proof) -> Bucket {
            
            info!("=== BORROW OPERATION START === ");
            info!("Loan pool size is: {}", self.loan_pool.amount());
            info!("Main pool size is: {}", self.main_pool.amount());

            // Get the data associated with the Borrowing NFT and update the variable values (in_progress=false)
            let non_fungible: NonFungible<BorrowingTicket> = ticket.non_fungible();
            let mut borrowing_nft_data = non_fungible.data();
            //check if no operation is already in place            
            assert!(!borrowing_nft_data.in_progress, "You have a borrow open!");
            assert!(
                self.main_pool.amount() > pool_low_limit(self.start_amount, self.main_pool_low_limit),
                "Main pool is below limit, borrowings are suspendend "
            );  
            // The ratio of requested liquidity.
            let ratio = xrd_requested * dec!("100") / self.main_pool.amount();            
            //check if loan is acceptable            
            //bucket size has to be between 3% and 12% of the main vault size
            let min_level: Decimal = calculate_level(self.min_ratio_for_borrow,self.main_pool.amount()); 
            let max_level: Decimal = calculate_level(self.max_ratio_for_borrow,self.main_pool.amount());
            assert!(
                ratio > calculate_ratio(self.min_ratio_for_borrow, self.main_pool.amount()),
                "Borrow is below the minimum level, actual minimum is: {} Min tokens you can borrow is {}", ratio, min_level
            );  
            assert!(
                ratio < calculate_ratio(self.max_ratio_for_borrow, self.main_pool.amount()),
                "Borrow is above the minimum level, actual maximum is: {} Max tokens you can borrow is {}", ratio, max_level
            );    

            // The amount of $xrd token to be repaid back (fee included)
            info!("Gettin from main pool xrd tokens size: {}", xrd_requested);
            //take $xrd from main pool
            let xrds_to_give_back = self.main_pool.take(xrd_requested);

            //calculate the fee for this operation
            let fee_value = xrd_requested*self.fee/dec!("100");
            //calculate the total amount the borrow has to repay
            let mut xrd_to_be_returned = xrd_requested + fee_value;
            borrowing_nft_data.in_progress = true;
            info!("Number of borrowings is: {} - L1 : {} - L2 : {}", borrowing_nft_data.number_of_borrowings, borrowing_nft_data.l1, borrowing_nft_data.l2);
            if l1_enabled(borrowing_nft_data.number_of_borrowings,10,20) {    
                borrowing_nft_data.l1 = true;
                println!("L1 reached ! bonus fee assigned ");
                xrd_to_be_returned -= xrd_requested*self.bonus_fee_l1/100;
            } else if l2_enabled(borrowing_nft_data.number_of_borrowings,10,20) {
                borrowing_nft_data.l2 = true;
                println!("L2 reached ! bonus fee assigned ");
                xrd_to_be_returned -= xrd_requested*self.bonus_fee_l2/100;                
            }            
            borrowing_nft_data.xrds_to_give_back = xrd_to_be_returned;
            info!("XRDs to be repaid back is: {}", borrowing_nft_data.xrds_to_give_back);
            // Update the data on that NFT globally         
            self.loan_admin_badge.authorize(|| {
                borrow_resource_manager!(self.borrowing_nft_resource_def).update_non_fungible_data(&non_fungible.id(), borrowing_nft_data)
            });            

            info!("New Loan pool size is: {}", self.loan_pool.amount());
            info!("New Main pool size is: {}", self.main_pool.amount());            

            //the bucket with the amount requested
            xrds_to_give_back
        }        

        /// Repay back XRD token 
        pub fn repay_money(&mut self, mut xrd_tokens: Bucket, ticket: Proof) -> Bucket {
            info!("=== REPAY OPERATION START === ");
            info!("Loan pool size is: {}", self.loan_pool.amount());
            info!("Main pool size is: {}", self.main_pool.amount());                
            // Get the data associated with the Borrowing NFT and update the variable values (in_progress=false)
            let non_fungible: NonFungible<BorrowingTicket> = ticket.non_fungible();
            let mut borrowing_nft_data = non_fungible.data();
            //check if no operation is in place            
            assert!(borrowing_nft_data.in_progress, "You have not a borrow open!");
            
            let xrd_returned = xrd_tokens.amount();
            
            //update the cumulative borrowings completed
            self.cumulative += xrd_returned;         
            info!("Cumulative loan value completed {} ", self.cumulative);   
            if xrd_returned >= borrowing_nft_data.xrds_to_give_back {
                //take back the tokens received
                self.main_pool.put(xrd_tokens.take(borrowing_nft_data.xrds_to_give_back));
                info!("All xrd tokens being repaid ! {} , token received in eccess {} " , 
                    borrowing_nft_data.xrds_to_give_back , (xrd_returned - borrowing_nft_data.xrds_to_give_back));                
                borrowing_nft_data.xrds_to_give_back = Decimal::zero();
                borrowing_nft_data.in_progress = false;
                let number_of_borrowings = 1 + borrowing_nft_data.number_of_borrowings;
                borrowing_nft_data.number_of_borrowings = number_of_borrowings;    
            } else  {
                borrowing_nft_data.xrds_to_give_back -= xrd_returned;
                info!("Some xrd tokens are to be repaid yet ! , you returned {}, but you missed by {} " , xrd_returned, borrowing_nft_data.xrds_to_give_back);
                //take back the tokens received even if partially
                self.main_pool.put(xrd_tokens.take(xrd_returned));
            }

            //mint the fee as lnd token and put in the loan vault
            let lnd_to_be_minted = ((self.fee*xrd_returned)/(dec!("100")+self.fee)).round(2,RoundingMode::TowardsPositiveInfinity);
            info!("Loan token to be minted : {}", lnd_to_be_minted);

            let new_tokens = self.loan_admin_badge.authorize(|| {
                borrow_resource_manager!(self.loan_resource_def).mint(lnd_to_be_minted)
            });
            self.loan_pool.put(new_tokens);

            // Update the data on that NFT globally
            self.loan_admin_badge.authorize(|| {
                info!("Updates Borrowing NFT ! num. = {} , in progress = {}", borrowing_nft_data.number_of_borrowings, borrowing_nft_data.in_progress );
                borrow_resource_manager!(self.borrowing_nft_resource_def).update_non_fungible_data(&non_fungible.id(), borrowing_nft_data);
            });

            info!("New Loan pool size is: {}", self.loan_pool.amount());
            info!("New Main pool size is: {}", self.main_pool.amount());

            //the bucket with the remainder, if any
            xrd_tokens
        }

        //returns the fee
        pub fn fee(&self) -> Decimal {
            return self.fee;
        }
        //returns the reward
        pub fn reward(&self) -> Decimal {
            return self.reward;
        }
        //returns the loan pool size
        pub fn loan_pool_size(&self) -> Decimal {
            return self.loan_pool.amount();
        }
        //returns the main pool size
        pub fn main_pool_size(&self) -> Decimal {
            return self.main_pool.amount();
        }
    
    }
}