# GitHub Discussions setup

Use the public [application forum](https://github.com/erp4us-org/erp4us/discussions) as the single source. Repository Discussions are already enabled. Do not copy the categories or posts into the profile or website repositories.

## Organization and categories

1. As an organization owner, open **erp4us-org → Settings → Discussions**. Select **Enable discussions for this organization**, choose **erp4us** as the source repository and click **Save**. Confirm that the organization and repository tabs show the same categories and discussions.
2. In the application forum, click the pencil next to **Categories**. Edit the existing categories in place, keeping their formats. Create only missing categories from the table; use **Open-ended discussion** for new categories.
3. Check each category URL against the matching `.yml` basename under [`.github/DISCUSSION_TEMPLATE`](../../.github/DISCUSSION_TEMPLATE). If GitHub generates a different slug, rename the file to the actual slug. After the forms land on the default branch, confirm that **New discussion → Get started** shows the expected form for all six categories.

| Category to use | Action | English description |
|---|---|---|
| Announcements | Keep announcement format | Project news, decisions and invitations from erp4us maintainers. |
| Introductions | Create | Introduce yourself, your interests and the work you would like to help shape. |
| General | Keep | Discuss erp4us topics that do not fit a more specific category. |
| Ideas & Feature Requests | Rename Ideas | Describe a problem or improvement and the outcome you want; a technical design is optional. |
| Business Requirements | Create | Explain who needs what, when and why, with an example and a measurable outcome. |
| Industry Knowledge | Create | Share anonymized workflows, handoffs, terminology and exceptions from your industry. |
| Architecture & RFCs | Create | Review architecture options, contracts, trade-offs and RFC proposals before decisions are made. |
| UI/UX & Design | Create | Share workflow, accessibility and usability feedback, or sketches using fictional data. |
| Localization | Create | Discuss translation, regional formats and local business conventions using fictional examples. |
| AI & Agent Integration | Create | Discuss agent workflows, permissions, human approval and audit requirements. |
| Community Polls | Rename Polls; keep poll format | Vote on discovery priorities and explain your choices; votes do not approve requirements. |
| Q&A | Keep question-and-answer format | Ask about using or contributing to erp4us and mark helpful answers. |
| Show and Tell | Rename Show and tell | Share demos, experiments and contributions without confidential or customer data. |

## Starter discussions

Search for each title before posting. Reuse an existing discussion on the same topic rather than duplicating it. From the repository's [New discussion chooser](https://github.com/erp4us-org/erp4us/discussions/new/choose), select the category, use the title and draft below, then select **Start discussion**. Append `*Starter post drafted with GitHub Copilot.*` to each published draft.

### Welcome to erp4us – ERP by everyone, for everyone

**Category:** Announcements

Welcome to the public meeting place for erp4us. We are building a self-hostable ERP that people from different industries and countries can help shape. Business users, industry experts, translators, designers and developers are equally welcome. You do not need to write code.

Introduce yourself in **Introductions**, share a fictional workflow in **Industry Knowledge**, or describe a need in **Business Requirements**. Search first and see our [contribution guide](https://github.com/erp4us-org/erp4us/blob/main/CONTRIBUTING.md) for other ways to help. English is our primary language, but other languages are welcome. Follow the [Code of Conduct](https://github.com/erp4us-org/erp4us/blob/main/CODE_OF_CONDUCT.md), never share confidential or customer data, and clearly label substantive AI-generated contributions. A discussion is not an approval or a release commitment.

### What is missing in today's ERP systems?

**Category:** Business Requirements

What business task is harder than it should be? Tell us who does it, what triggers it, where the current process breaks down and what a good outcome looks like. If there is a workaround, how often do you need it and what does it cost in time or effort? Use a fictional example. No technical design is needed.

### Tell us about your industry and business processes

**Category:** Industry Knowledge

Pick one workflow: what industry is it in, which roles are involved and which handoffs or exceptions matter? For example, a customer orders ten units but only six are available. When would you split the order, backorder the remainder, substitute an item or ask for approval? Which parts might another industry handle differently? Please use fictional data.

### Designing a universal product and material model

**Category:** Architecture & RFCs

What does your business call a product, article or material, and when are those different concepts? Share a fictional example involving variants, service items, packaging and units, or batch and serial tracking. Which facts stay stable when a product changes? Which are specific to your industry? This is discovery, not an accepted model or decision.

### What should a modern ERP user interface look like?

**Category:** UI/UX & Design

Choose a task you do often, such as finding an article, entering an order or correcting a mistake. Tell us where you lose time or context, what confirmation you need and whether keyboard navigation, assistive technology or mobile use matters. What observable result would show that the interface is better? Sketches are optional and should use fictional data.

### How should erp4us support international businesses?

**Category:** Localization

Tell us which language, locale or region matters and give a fictional example where translation, number or date formats, currencies, units or document conventions affect the workflow. Are several locales used in the same organization? Distinguish shared needs from regional or regulatory exceptions and link public references if available.

### Building an agent-native ERP platform

**Category:** AI & Agent Integration

Which task would you let an AI assistant help with, and where must a human remain in control? Give a fictional scenario and distinguish reading information, proposing changes and executing actions. What permissions, preview, approval and audit evidence would make that workflow acceptable? What should happen if the agent is unavailable or wrong?

### Which business capabilities should we prioritize?

**Category:** Community Polls

Use the category's **New discussion** form, fill in **Poll question** and **Poll options**, and click **Start poll**. The question is “Which business capability should erp4us investigate first?” Use these options: **Product and material master data**, **Sales and order fulfillment**, **Inventory and procurement**, **Service and project work**, and **International workflows and localization**.

Body: “Vote for the area where research would help your work most. In a comment, describe the outcome you need (using fictional data). Results inform discovery, not requirement approval or release commitments.”

## Pins and verification

Open the welcome, ERP gaps and industry workflows discussions. In the right sidebar choose **Pin discussion**, then click **Pin discussion** to confirm. Check the organization forum for the same entry points. If pins are shown separately in that view, pin them there as well. If the global pin limit has already been reached, preserve the welcome pin and use **Pin discussion to CATEGORY** for the others rather than removing unrelated pins.

Recheck the forum while signed out, then open it as a contributor and preview the six discussion forms. Do not submit test posts. The [community requirements workflow](../requirements/community-workflow.md) explains how maintainers turn feedback into traceable proposals without automatically approving requirements or opening issues.
