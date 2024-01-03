# Overview

The Cookly CLI tool helps to automate the process of creating infrastucture, connecting to and provisioning EC2 instances, and connecting to databases to run queries.

---

## Installation

#### Firstly Install the Github CLI Tool: [HERE](https://cli.github.com)**

**On Mac**
Install via:
```bash
brew install gh
```


**On Debian or Ubuntu Linux**
Install via:
```bash
type -p curl >/dev/null || (sudo apt update && sudo apt install curl -y)
curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
&& sudo chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
&& echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
&& sudo apt update \
&& sudo apt install gh -y
```


**Once it has been installed successfully** 
Run through authenticating the Github CLI with your github account. 
To do this run:

```
gh auth login
>Q: What account do you want to log into? 
 A: GitHub.com
>Q: What is your preferred protocol for Git operations on this host? 
 A: HTTPS
>Q: Authenticate Git with your GitHub credentials? 
 A: Yes
>Q: How would you like to authenticate GitHub CLI? 
 A: Login with a web browser
```
 
**Run the following in your terminal to install the cookly-cli**

gh release download v1.0.0 -R bracketengineering/cookly-cli -D ./releases/
