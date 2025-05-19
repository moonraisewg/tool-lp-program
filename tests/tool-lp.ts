import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { ToolLp } from "../target/types/tool_lp";
import { withdraw } from "./utils";

describe("withdraw only test", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const owner = anchor.Wallet.local().payer;
  const program = anchor.workspace.ToolLp as Program<ToolLp>;

  const confirmOptions = {
    skipPreflight: true,
  };

  it("withdraw from existing LP", async () => {
    const cpSwapPoolState = {
      ammConfig: new anchor.web3.PublicKey("POOL_CONFIG_PUBLIC_KEY"),
      token0Mint: new anchor.web3.PublicKey("TOKEN0_MINT_ADDRESS"),
      token1Mint: new anchor.web3.PublicKey("TOKEN1_MINT_ADDRESS"),
      token0Program: new anchor.web3.PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
      token1Program: new anchor.web3.PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
    };

    const withdrawAmount = new BN(12_570_000); // rút toàn bộ 0.01257 LP

    const tx = await withdraw(
      program,
      owner,
      cpSwapPoolState.ammConfig,
      cpSwapPoolState.token0Mint,
      cpSwapPoolState.token0Program,
      cpSwapPoolState.token1Mint,
      cpSwapPoolState.token1Program,
      withdrawAmount,
      new BN(100000), // min token0
      new BN(100000), // min token1
      confirmOptions
    );

    console.log("Withdraw TX:", tx);
  });
});
