---
title: "nocodo: A Complete Introduction"
description: "Learn the fundamentals of nocodo - a structured approach to AI-assisted development that focuses on user flows and business logic."
tags: ["nocodo", "AI development", "methodology"]
---

I've been vibe coding for the last two to three months now. On a daily basis, I've been using Claude Code, Gemini CLI, a little bit of Repl.it, Bolt, Lovable, and many other tools in between. I use primarily Claude Code and I've been quite happy with the results.

## Why Vibe Code at All?

Vibe coding has become quite popular among a lot of enthusiasts, as well as engineers. But for me, what stands out is how productive it has become for me on a daily basis, where I can take a step back from the actual code in the software development process, and rather focus on what I want to build and why I want to build it. So, the actual intent of the software—the problem it is solving. And now, with this voice-based thought process sharing that I do, it has become even easier for me to think through the user flow.

### Empowering Non-Technical Users

And I think that's something that really helps people, particularly non-technical audiences, to share what they want to do, why they want to—like, what is the problem they are solving, why are they solving it, and what is the user journey. So, they focus—so we can focus on the user journey, and with a little bit of technical guidance, we can get fairly productive software out. Of course, how productive—what is the quality of that output software—depends a lot on how much scaffolding we give, what is the prompt, what are we asking the AI agents to build, how are we testing the software, how testable the software is going to become, what we have thought about deploying the software, and so on and so forth.

### Understanding the Challenges

So, these definitely are there as more like open challenges. So, it's not as easy as just sharing your thoughts as voice, just talking about the user journey and, you know, brilliant software will come out on the other side. Sometimes the software may itself—like, because the user journey itself may be complicated, the outcome software may also be fairly complex and can be very brittle, for lack of a better phrase.

And if it is brittle—by brittle, I mean it's just buggy. And what happens is that if the software is built upon a foundation that is brittle, of course, any new features you add on top of it just kind of make it more and more complicated and you will struggle to keep it maintainable at a certain point. Okay, so that's—that to me is the why—like, it definitely gives me a lot of benefit.

### Real-World Business Applications

I get a lot out of it and I feel particularly if it is for a non-technical user—like if I'm a business owner who does not know much about software engineering—with a little bit of homework, I can get so much done, like my entire work automation of certain business practices. Let's say if I'm a small baker and whatever process I need to go through to maybe go online and take orders or maybe in-shop, you know, allowing me to take orders from customers or to be able to take customization orders from customers, these would become much easier—like simple forms, getting the data into a database, replying back and forth, allowing the user to specify more details if I need them, that kind of back-and-forth communication, which at this point, we need some kind of SaaS to describe those things—like to do those things. But I think we are already at a point where we can build this kind of custom software without depending on a SaaS. It could be simply hosted on a server that I rent for just five to ten dollars a month, and that software would be sufficient for a small business to run a bunch of such operational software, which is really great.

### Cost-Effective Development

So from order tracking to allowing inventory management—some basic inventory management—to some tool for internal purposes, like if I have a five- or six-person team, to have that kind of collaboration within the team, so all of us can, you know, use some kind of this very rudimentary web-based app on our phones. All of this can just happen by me describing my flows. Of course, I'm not saying this is one software—I could build multiple pieces of software—but the main benefit is that now the prices of building this are crashing, really crashing.

Like, for the most part I use a Claude Pro account—so that's Anthropic Claude—and I use it only with the twenty-dollar-per-month account, plus a bit of taxes on top, and that is more or less enough for the whole month's worth of software development work I do. And I work on a ton of projects, and I'm not even trying to extract massive value out of it by waking up late at night, because they have these hourly quotas—so I'm not even trying to extract value by waking up at night and, you know, using those hourly quotas in the middle of the night. I'm not even trying that. I'm just doing normal hours.

At twenty dollars a month, which is becoming more and more standard, I think we can get enough value to run a small business, and that's a huge benefit, because that allows me to do order management, inventory management, all these custom software, team management, and stuff like that—my custom, maybe HR. So a small business—whatever software we may need, we will be able to create that.

## What is nocodo?

So vibe coding is basically, as Andrej Karpathy has mentioned—he's this person from, you know, the original OpenAI team, brilliant fellow, he has a lot of instructional videos on YouTube, please go follow him, absolutely fantastic person—so according to him, vibe code—like from how I understand, loosely—is like, you depend, you really depend, you go all in and depend on the AI models to structure your code and you take a step back.

### The Core Philosophy

You don't look into the code, you don't write the code. You're not an engineer, or if you are an engineer, you're basically taking a step back from that daily coding. Like when I started off vibe coding, I was still reviewing some of the code. Now I don't even review the output code line by line, and that's probably like a massive shift in my mindset—like that's really, really important to take note of.

And to me, that is vibe coding—like I am structuring the project just like how I would structure if I had a development team, and that is something that I'm still doing and it applies to vibe coding very well because you still want to break down some of those higher-level thoughts, some of those complex user flows, the journeys—who is the customer, who's going to order. Let's say, you know, simple business applications—like who's the internal user, who's the external user, who is going to see what, who can have administrative rights. It's important to think through those things, but other than that and a little more technical details that we'll go into later on, you don't really write code, you don't really read code—you test the output software each step at a time, but that's about it.

So that to me is vibe coding—you're not looking at vibe coding, you're really reliant on the AI models to produce code that is testable and hopefully maintainable, so you don't crash—you know, the software does not crash every second day.

## How to Vibe Code

Now how I have been doing for the last, you know, two to three months—like I've said—has evolved a bit. I initially started off with being more detail-oriented—like I would write down a lot of text and this text I would then send to Claude or to—by Claude I mean the web interface. You can also use Claude Code directly on your computer, you can use Gemini CLI or any of these terminal-based tools. But yeah, if you're not comfortable with terminal-based tools, don't be afraid—there are so many tools out there and the tooling is getting better, so don't worry about that right now. It's more the higher-level ideation that you should be thinking about—like what I'm talking about is the higher-level structuring and ideation.

### Starting with Problem Definition

So what I do is I start with thinking through what is the problem that I'm trying to solve in this product. I basically take the high-level idea, okay, and I put it into a document which is my readme.md (.md is for markdown files). So I take this high-level idea and I write like a few paragraphs about it.

### Voice Notes Approach

The one thing I'm changing now—and this has been very recent—is that I take voice notes. It's very interesting that you can just record your voice, and I think voice is even more fluid when it comes to expressing yourself. In a way, it just feels a little more natural to me than typing it out, so I prefer voice. I can record on my phone, I can record on my computer. I have a microphone, like a semi-professional microphone. So whatever it is, you can feel free to use whatever you have—even just your basic phone microphone is great.

And then you can use any of these free text-to-speech software on your phone—like there are many apps which are completely free—or on your laptop you can just go on the web browser and search for a few. There are many of them which are free—like the one I'm using right now is called TurboScribe. It gives you three speech-to-text per day and each can be like a 30-minute-long recording, so that is great.

### Concrete Example: Grocery Marketplace

So let's take a concrete example—like if I were to describe a grocery shopping—this is an idea that I've been chasing for a while. So the high-level idea I'm just stating right now and you could take inspiration from this, but you know, solve for any other problem that you're doing—so I want a grocery software where I can go online and I can list my grocery items that I need, so that's going to be my grocery list, right?

Okay, so any user, any consumer, let's say, can list their grocery items—let's call them grocery lists. Now users don't get to see each other's grocery lists because that would not be of any use—it doesn't really solve a problem. The grocery suppliers, the actual stores nearby in my locality, they sign up to be able to see the lists. Of course, the users are already giving the consent that this list of theirs that they have created is going to be seen by nearby shops.

#### Defining Business Rules

Now, what do we say—what is nearby? Now here is some of those things that you have to think through in vibe coding, but also just like in software definition anyway—what is that locality? Like how do you define locality for a grocery list to be exchanged? So in my case when I was thinking about the software, I wanted the regionality to be—the locality, sorry—to be like five kilometers, or I don't know whatever it is in miles, maybe three miles and no more than that. But it could be a little more depending on which country you're from, and you could think of, okay, maybe it will be flexible depending on country, whatever. So whatever you think it should be, think about it a little and document that.

You can document it like we said—you can document it by voice, you can document it in text, whatever it is, or then convert the voice into text. That does not matter. What matters is that you document these little things, okay?

#### Supplier Validation

So from there I would say, now who are these grocery suppliers, right? How do they come to the system? So I have to think again—okay, I want to validate these people, and how would they be validated? You have to give some kind of a business license—this will depend on the country—and then you'll have to give me, the administrator—this company running this grocery mechanism, the online marketplace, let's say—you'll have to give us also some kind of an identity proof and your Google Maps location, blah blah blah. Think through this again. This thinking is important. This thinking has nothing to do with AI—that's what I want to make sure that we get right. This has nothing to do with AI. This is just thinking through the business problems.

#### Identifying User Types

Then what I would do is think, okay, so what kind of users now are we talking about? We have the consumer, we have the supplier, and we have the administrator, because without the administrator, how am I going to validate the supplier, right? Who is the supplier? Can the supply-side people, the grocery owners—sorry, the store owners—can they immediately get access just by signing up with a username and password or an email and password? No, of course they cannot, so they have to go through the validation process. How do I validate them? So I need an administrative section.

So now the immediate question comes, and this is where a little bit of the technology starts creeping in—so the user has a side, the administrator has a side, and the supplier has a side. Generally in a marketplace website we call them the demand side and the supplier side, and of course the administrator in between. The demand is whoever is paying money to get something, so the consumers here are the demand side.

### AI-Assisted Architecture Planning

So now wherever we have this point where there are multiple—seemingly multiple different apps—we can ask AI, hey, does it make sense to create different apps? Because this is my general product idea. Of course, this happens only once you have laid out your product idea, and then you push—then you give access to this to Claude or to Gemini or to OpenAI, ChatGPT, whoever. So save this as a file, tell them, hey, please analyze this and now ask me clarification questions. Or you can ask, hey, do you think that these three should be one app on the browser, on the mobile phone, or do you think they should be separate apps? And different—depending on the context, you can debate with them, you can negotiate with them.

Generally speaking, they will come up with answers which will make you think a little more, and maybe they will also have to think a little more, and this kind of goes on back and forth. Like in this grocery case, my way of thinking would be that these should be separate apps, and I'll tell you why—because the way the administrative app works, who gets access, what is seen—the access is totally different. Because where all this data is saved, the logic and everything is what we typically call back-end.

#### Understanding Data Flow

Of course, when consumers share their grocery list, that data has to go somewhere, right? And when store managers see the grocery list, they have to see that from that same store of data. And when the administrator—let's say me—logs in and validates a store owner, I have to see the requests coming from store owners. Store owners have to sign up somewhere, consumers have to sign up somewhere, and I have to log in somewhere or even sign up somewhere, right?

Now you see what I'm doing here—I'm trying to be very explicit about the user journey. And right now in such a simple grocery tool—like a marketplace for just nearby groceries to be given out as a request and then fulfilled by maybe a grocery store—looks simple when you describe it. Like this is where I think we have to shift into the mindset of building good software with vibe coding—that you cannot just declare this as an elevator pitch. Like if I did this in like a 60-second "hey, I want to build blah blah blah," this will not come out as good software.

But the more you think through the user flow, now suddenly we know that there are three kinds of users, suddenly we are thinking maybe there are three kinds of front-end apps. These are called usually front-end apps because they are facing the user—so front-end apps. Even the administrator is a user. And then there's the back-end side of this where this data, the database, etc., the permissions, the controls will all stay, right? So immediately now we know there are four at least pieces of this puzzle—maybe there are more, I don't know, but this clarification is very important. So laying this down is very important.

### Technical Foundation

So first we describe the product, describe the problem, describe the user journeys, etc. The next step is where it gets a little more technical. But on a very high level, the grounding rules that you can use—the very basic principles that you can use—is try to ask AI, hey, what are the simplest existing software that I could use to build some of these things?

So let's say you figured out that there are four sections like we discussed in this grocery marketplace thingy, and then you could ask AI, hey, what are the basic, good, well-maintained—really well-maintained—software out there, open-source software on which I can build this to get my product—build my MVP—built? The moment you ask for that, there is a big chance that your Claude Code or your Gemini or your OpenAI—all of these will come back with examples like, hey yeah, you should use Ruby on Rails, or maybe Node.js, or sorry, Next.js, or maybe Python with Django or Python with FastAPI, and then maybe build a React website on top, etc., or maybe build one React website for all of these, or maybe separate ones for different user types, etc. It will give you those choices.

Now you may not understand all of those choices, and that's okay. That help will come both from a little bit of your own homework—like you can go ahead and read a little bit of material—and much of it will come from just negotiating with AI.

### Task Breakdown and Project Management

And then comes the last step. The last step is to break down all of these high-level pieces—so the documentation about the product, where is the user, what is the user, who's the user, their journey, blah blah blah, and then the documentation of the basics of the technical—let's say grounding—like I have these preferences, I want to be able to run this software on the cloud, I want to host it here, I want this kind of a service provider, or I want this kind of—you know, I'm hoping for XYZ number of users to be coming and my software being able to serve for that. It's about making sure that they are reality-checked.

And then the last step is to break them down into small tickets—these you could say tasks, tickets, issues. They are more or less the same name—like the same meaning—is that you basically break them down into palatable chunks. And this breaking down also is done with AI, by the way. So AI tools are great at breaking these high-level topics down into small chunks. And then you say, hey, save these into a task management software.

So if you have used any kind of project management software—in software development we use project management software a lot—like I don't think there is any company, any software development company who does not use—unless they love chaos—they probably use a project management software. So you should also do that because that will enable you to visualize this product as a whole now as small pieces, so that you know, okay, now I see I need this app, this part to be built, then that part can interact with that part, and then there will be my database, and these are the tasks—one step, two step, three steps, etc.—that AI will be accomplishing. These tasks.

### Work Plans and Dependencies

So now once you have that high-level picture, right, you can ask AI to also create a work plan. The work plan will enable you to visualize what is the dependency, because in these software engineering tasks, just like in normal tasks—let's say you are building a house, right—there is a dependency, right? You do not want to—do you want to buy your plumbing pipes before you have any of the foundation laid out? Probably not, right, because the foundation comes first. Maybe it takes a while—sure, you can buy everything in advance, but it'll just be lying there.

So that kind of thinking—the process of approaching a problem to build something—needs what we generally call like a dependency list or some thinking around the dependencies, right? What is dependent—which step is dependent on which previous step? And you can ask AI to create that, and generally I save it as like a work plan in my project. And then I simply go—so the work plan is simply pointing to those issues we created like I mentioned—and now you go step by step.

At least the first phase of the project, ask AI when you are creating—like what is the work plan—like if I want to just see the MVP of my product, just the minimum viable product? Okay, in fact, your entire first approach should be just focused on the MVP. You can again negotiate with AI to remove every fluff, everything that you feel like, no, I don't need this because this is not minimal. Anything that is not minimal—even like good-looking UI and stuff like that—you can do later. First build something that works, okay?

### Implementation

So now we got the steps, we got the work plan. Now all you have to do is get it started. So you take all of this and you ask your Claude Code or your Gemini CLI or like OpenAI's Codex CLI or Qwen Code—this is the open-source model and its CLI from Alibaba Cloud—so there are tons of them, and there are many others. You can use Cursor, you can use Amazon Q.

Basically what you now do is that you have your tasks laid out somewhere—I would suggest using GitHub for the issues or a nice project management software like Linear. But now all you have to do—and this is what I do—I say, hey Claude, take issue number one, look at the current state of the project, and try it out—try to implement it. That's it. That's how simple it is.

Of course, there are a little more technical aspects that I usually have in my projects—I'm not going to discuss them right now, but they are not too hard, they're not too difficult to enable for any user, and you don't need technical skills, you don't need programming skills to even use those ideas.

## Getting Started: Practical Advice

But that's about it—those are the high-level structures to be able to—when I'm saying try this out, try building a small grocery application—like a small—like even your own private list. Maybe you just want a grocery list for you and your family and there is no supply side at all—you don't care about who's going to supply, this is just for private use. Go ahead and build that.

Build a few apps for yourself. Don't go all the way crazy to like, hey, I want to build a whole iOS app. No, try to build a small web app. These days any AI model will probably generate a very mobile-friendly web app, so you don't have to worry about, oh, I'm not going to be able to see it on a phone. No, you're going to see it on your phone very well.

The main thing is that when you generate software that you want to go and actually use in your business, remember there is this entire part which is like, hey, where do I put this now? Where will it execute? Where will I host this? Where will the backups go? We have not discussed any of these, but again, this is just to set the structure in your mind—just to try out small apps like I said—small apps which you can probably use just to keep track of what groceries to buy or what tasks to do, what chores to do in your family. Start with those small apps.

Remember, if you start small, if you stay focused, you can always expand. The most important thing to understand is how to become structured with AI—not looking at code, not knowing code. That's the most important part. We are not spending time babysitting code or writing code on a daily basis. Most of us—many of us probably watching this content—do not even know how to code, so totally out of the question, right? That's the main thing that I'm aiming for.

Hopefully this helps some of you. You can reach out to me—I'm also posting my upcoming sessions where I'll be doing some live coding, so feel free to join them. But otherwise, yeah, keep building. Thanks!