FROM node:16-alpine
WORKDIR /dockerfile-generator
COPY . .
RUN npm i && npm i --location=global .
ENTRYPOINT ["dockerfile-generator"]