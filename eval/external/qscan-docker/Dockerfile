FROM node:20-slim
# Install without running package lifecycle scripts.
RUN npm install -g --ignore-scripts @quantakrypto/qscan@0.12.0 \
 && npm ls -g --depth=1 @quantakrypto/qscan
USER node
WORKDIR /scan
ENTRYPOINT ["qscan"]
