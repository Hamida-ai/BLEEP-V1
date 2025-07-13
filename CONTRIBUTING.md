🤝 Contributing to BLEEP

Welcome! 👋  
We're excited that you're interested in contributing to BLEEP — a self-healing, AI-native, quantum-secure blockchain ecosystem.

This document outlines how you can participate in shaping BLEEP, what contributions are welcome, and the standards we expect to maintain a high-quality, community-driven platform.

---

📌 About BLEEP

BLEEP is a next-generation blockchain built for resilience, scalability, and intelligence. It features:
- A self-amending protocol
- AI-native infrastructure
- Quantum-resistant cryptography
- Modular architecture (VM, SDKs, smart contracts, docs)

We welcome contributors from all backgrounds — developers, researchers, writers, designers, and enthusiasts.

---

🧠 Contribution Areas

We invite contributions in the following areas:

Module              | Description
--------------------|-------------------------------
/core               | Blockchain protocol and consensus
/smart-contracts    | Base smart contracts (governance, assets, etc.)
/sdk                | Developer SDKs (Rust, TypeScript, etc.)
/vm                 | BLEEP Virtual Machine
/docs               | Whitepapers, guides, tutorials
Bug Reports         | Issues, edge cases, protocol behavior
Feature Requests    | Ideas to enhance any part of BLEEP
UI/UX Feedback      | For the wallet, explorer, or future apps

---

⚙️ Getting Started

1. Fork the Repository

   Click the “Fork” button on the GitHub repo: https://github.com/BleepEcosystem/BLEEP-V1, then:

   git clone git@github.com:your-username/BLEEP-V1.git
   cd BLEEP-V1

2. Create a Feature Branch

   git checkout -b feat/your-feature-name

3. Make Your Changes

   Follow conventions for:
   - Code formatting
   - Documentation
   - Tests
   - Licensing (SPDX headers, etc.)

4. Test Everything

   Run tests relevant to the module you're working on:

   # Rust (core, VM)
   cargo test

   # Solidity (smart contracts)
   forge test

   # SDK (JavaScript/TypeScript)
   npm test

5. Commit and Push

   git commit -m "feat: short description of your change"
   git push origin feat/your-feature-name

6. Submit a Pull Request

   - Go to your fork on GitHub
   - Click “Compare & Pull Request”
   - Provide a clear description of what your PR does
   - Reference related issues if any (e.g. Closes #123)

---

✅ Contribution Guidelines

- Keep Pull Requests small and focused
- Add or update documentation when relevant
- Write meaningful commit messages
- Use SPDX license headers in all new files
- Be patient, respectful, and constructive in code reviews

---

🛡️ Code of Conduct

We follow a Code of Conduct to ensure BLEEP remains a welcoming, respectful community for everyone.  
Please be kind, curious, and inclusive.

---

📜 Licensing

By contributing, you agree that your code will be published under the licenses used in this repository:

Directory           | License
--------------------|------------------------
/core               | Apache-2.0
/smart-contracts    | GPLv3
/sdk                | MIT
/vm                 | BSL 1.1 → Apache 2.0 (2028)
/docs               | CC-BY-4.0

---

🙌 Thank You

Your time and contributions help BLEEP evolve into a powerful public good.  
Together, we are building the future of intelligent, resilient, decentralized systems.

BLEEP ≡ Evolve Everything. 
