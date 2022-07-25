FROM node:16-alpine
WORKDIR /dig
COPY . .
RUN npm i && npm i --location=global .
ENTRYPOINT ["dig"]