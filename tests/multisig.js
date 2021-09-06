const anchor = require("@project-serum/anchor");
const assert = require("assert");

describe("social recorvery", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.SerumMultisig;

  it("is just a proxy when sender is the wallet's signer", async () => {
    const socialRecovery = anchor.web3.Keypair.generate();
    const [
      socialRecovery,
      nonce,
    ] = await anchor.web3.PublicKey.findProgramAddress(
      [socialRecovery.publicKey.toBuffer()],
      program.programId
    );

    const socialRecoverySize = 200; // Big enough.

    const allyA = anchor.web3.Keypair.generate();
    const allyB = anchor.web3.Keypair.generate();
    const allyC = anchor.web3.Keypair.generate();
    const allyD = anchor.web3.Keypair.generate();
    const allies = [allyA.publicKey, allyB.publicKey, allyC.publicKey];

    const threshold = new anchor.BN(3);
    await program.rpc.createSocialRecovery(allies, threshold, nonce, {
      accounts: {
        socialRecovery: socialRecovery.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      instructions: [
        await program.account.socialRecovery.createInstruction(
          socialRecovery,
          socialRecoverySize
        ),
      ],
      signers: [socialRecovery],
    });

    let socialRecoveryAccount = await program.account.socialRecovery.fetch(socialRecovery.publicKey);
    assert.strictEqual(socialRecoveryAccount.nonce, nonce);
    assert.ok(socialRecoveryAccount.threshold.eq(new anchor.BN(2)));
    assert.deepStrictEqual(socialRecoveryAccount.allies, allies);
    assert.ok(socialRecoveryAccount.allianceSeqno === 0);

    const pid = program.programId;
    const accounts = [
      {
        pubkey: socialRecovery.publicKey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: socialRecoverySigner,
        isWritable: false,
        isSigner: true,
      },
    ];
    const newAllies = [allyA.publicKey, allyB.publicKey, allyD.publicKey];
    const data = program.coder.instruction.encode("set_allies", {
      allies: newAllies,
    });

    const transaction = anchor.web3.Keypair.generate();
    const txSize = 1000; // Big enough, cuz I'm lazy.
    await program.rpc.executetransaction(pid, accounts, data, {
      accounts: {
        socialRecovery: socialRecovery.publicKey,
        transaction: transaction.publicKey,
      },
      instructions: [
        await program.account.transaction.createInstruction(
          transaction,
          txSize
        ),
      ],
      signers: [transaction, allyA],
    });

    assert.ok(txAccount.programId.equals(pid));
    assert.deepStrictEqual(txAccount.accounts, accounts);
    assert.deepStrictEqual(txAccount.data, data);
    assert.ok(txAccount.socialRecovery.equals(socialRecovery.publicKey));
    assert.deepStrictEqual(txAccount.didExecute, false);
    assert.ok(txAccount.allySetSeqno === 0);

    // Other ally approves transactoin.
    await program.rpc.approve({
      accounts: {
        socialRecovery: socialRecovery.publicKey,
        transaction: transaction.publicKey,
        ally: allyB.publicKey,
      },
      signers: [allyB],
    });

    // Now that we've reached the threshold, send the transactoin.
    await program.rpc.executeTransaction({
      accounts: {
        socialRecovery: socialRecovery.publicKey,
        socialRecoverySigner,
        transaction: transaction.publicKey,
      },
      remainingAccounts: program.instruction.setOwners
        .accounts({
          socialRecovery: socialRecovery.publicKey,
          socialRecoverySigner,
        })
        // Change the signer status on the vendor signer since it's signed by the program, not the client.
        .map((meta) =>
          meta.pubkey.equals(socialRecoverySigner)
            ? { ...meta, isSigner: false }
            : meta
        )
        .concat({
          pubkey: program.programId,
          isWritable: false,
          isSigner: false,
        }),
    });

    socialRecoveryAccount = await program.account.socialRecovery.fetch(socialRecovery.publicKey);

    assert.strictEqual(socialRecoveryAccount.nonce, nonce);
    assert.ok(socialRecoveryAccount.threshold.eq(new anchor.BN(2)));
    assert.deepStrictEqual(socialRecoveryAccount.allies, newOwners);
    assert.ok(socialRecoveryAccount.allySetSeqno === 1);
  });
});
