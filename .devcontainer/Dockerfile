FROM paritytech/contracts-ci-linux

RUN apt-get update 

# Install ssh
RUN apt-get install openssh-client -y

# Install rust version for zero node
RUN rustup toolchain install nightly-2020-10-05 && \
    rustup target add wasm32-unknown-unknown --toolchain nightly-2020-10-05

# Install zsh
RUN apt-get install zsh wget -y
RUN wget https://github.com/robbyrussell/oh-my-zsh/raw/master/tools/install.sh -O - | zsh || true 

# Install yarn and latest version for node
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.35.3/install.sh | bash  || true
RUN bash -c 'source $HOME/.nvm/nvm.sh   && \
    nvm install node                    && \
    npm install -g yarn && \
    npm install --prefix "$HOME/.nvm/"'
RUN echo 'export NVM_DIR="$HOME/.nvm" && [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"' >> ~/.zshrc

# Clean Up
RUN apt-get clean -y && rm -rf /var/lib/apt/lists/*
